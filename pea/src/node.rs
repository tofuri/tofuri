use crate::{
    blockchain::Blockchain,
    heartbeat, http,
    p2p::{gossipsub, multiaddr, request_response, Ratelimit},
    p2p::{Behaviour, OutEvent},
    util,
};
use colored::*;
use libp2p::{
    core::{connection::ConnectedPoint, either::EitherError, upgrade},
    futures::StreamExt,
    gossipsub::{error::GossipsubHandlerError, GossipsubEvent, IdentTopic, TopicHash},
    identity, mdns, mplex, noise,
    request_response::{RequestResponseEvent, RequestResponseMessage},
    swarm::{ConnectionHandlerUpgrErr, ConnectionLimits, SwarmBuilder, SwarmEvent},
    tcp, Multiaddr, PeerId, Swarm, Transport,
};
use log::{debug, error, info, warn};
use pea_address::address;
use pea_core::*;
use pea_db as db;
use pea_key::Key;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::Error,
    num::NonZeroU32,
    time::Duration,
};
use tempdir::TempDir;
use tokio::net::TcpListener;
use void::Void;
type HandlerErr =
    EitherError<EitherError<EitherError<EitherError<Void, Error>, GossipsubHandlerError>, ConnectionHandlerUpgrErr<Error>>, ConnectionHandlerUpgrErr<Error>>;
pub struct Options<'a> {
    pub tempdb: bool,
    pub tempkey: bool,
    pub mint: bool,
    pub time_api: bool,
    pub trust: usize,
    pub pending: usize,
    pub ban_offline: usize,
    pub time_delta: u32,
    pub max_established: Option<u32>,
    pub tps: f64,
    pub wallet: &'a str,
    pub passphrase: &'a str,
    pub peer: &'a str,
    pub bind_api: String,
    pub host: String,
    pub dev: bool,
    pub timeout: u64,
}
pub struct Node {
    pub p2p_swarm: Swarm<Behaviour>,
    pub p2p_message_data_hashes: Vec<Hash>,
    pub p2p_connections: HashMap<Multiaddr, PeerId>,
    pub p2p_ratelimit: Ratelimit,
    pub p2p_unknown: HashSet<Multiaddr>,
    pub p2p_known: HashSet<Multiaddr>,
    pub p2p_host: String,
    pub p2p_ban_offline: usize,
    pub blockchain: Blockchain,
    pub heartbeats: usize,
    pub lag: f64,
    pub tps: f64,
    pub bind_api: String,
    pub mint: bool,
    pub max_established: Option<u32>,
    pub tempdb: bool,
    pub tempkey: bool,
    pub dev: bool,
}
impl Node {
    pub async fn new(options: Options<'_>) -> Node {
        let key = Node::key(options.tempkey, options.wallet, options.passphrase);
        info!("Address {}", address::encode(&key.address_bytes()).green());
        let db = Node::db(options.tempdb);
        let blockchain = Blockchain::new(db, key, options.trust, options.pending, options.time_delta);
        let p2p_swarm = Node::swarm(options.max_established, options.timeout).await.unwrap();
        let p2p_known = Node::known(&blockchain.db, options.peer);
        Node {
            p2p_swarm,
            blockchain,
            p2p_message_data_hashes: vec![],
            heartbeats: 0,
            lag: 0.0,
            tps: options.tps,
            p2p_unknown: HashSet::new(),
            p2p_known,
            p2p_connections: HashMap::new(),
            bind_api: options.bind_api,
            p2p_host: options.host,
            mint: options.mint,
            p2p_ban_offline: options.ban_offline,
            max_established: options.max_established,
            tempdb: options.tempdb,
            tempkey: options.tempkey,
            dev: options.dev,
            p2p_ratelimit: Ratelimit::default(),
        }
    }
    fn db(tempdb: bool) -> DBWithThreadMode<SingleThreaded> {
        let tempdir = TempDir::new("peacash-db").unwrap();
        let path: &str = match tempdb {
            true => tempdir.path().to_str().unwrap(),
            false => "./peacash-db",
        };
        db::open(path)
    }
    fn key(tempkey: bool, wallet: &str, passphrase: &str) -> Key {
        match tempkey {
            true => Key::generate(),
            false => pea_wallet::util::load(wallet, passphrase).unwrap().3,
        }
    }
    async fn swarm(max_established: Option<u32>, timeout: u64) -> Result<Swarm<Behaviour>, Box<dyn std::error::Error>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Peer id {}", local_peer_id.to_string().cyan());
        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&local_key).expect("Signing libp2p-noise static DH keypair failed."))
            .multiplex(mplex::MplexConfig::new())
            .timeout(Duration::from_millis(timeout))
            .boxed();
        let mut behaviour = Behaviour::new(local_key).await?;
        for ident_topic in [
            IdentTopic::new("block"),
            IdentTopic::new("blocks"),
            IdentTopic::new("stake"),
            IdentTopic::new("transaction"),
            IdentTopic::new("multiaddr"),
        ]
        .iter()
        {
            behaviour.gossipsub.subscribe(ident_topic)?;
        }
        let mut limits = ConnectionLimits::default();
        limits = limits.with_max_established_per_peer(Some(1));
        limits = limits.with_max_established(max_established);
        Ok(SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id)
            .connection_limits(limits)
            .build())
    }
    fn known(db: &DBWithThreadMode<SingleThreaded>, peer: &str) -> HashSet<Multiaddr> {
        let mut known = HashSet::new();
        if let Some(multiaddr) = multiaddr::filter_ip_port(&peer.parse::<Multiaddr>().unwrap()) {
            known.insert(multiaddr);
        }
        let peers = db::peer::get_all(db);
        for peer in peers {
            if let Some(multiaddr) = multiaddr::filter_ip_port(&peer.parse::<Multiaddr>().unwrap()) {
                known.insert(multiaddr);
            }
        }
        known
    }
    pub fn filter(&mut self, data: &[u8], save: bool) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize().into();
        if self.p2p_message_data_hashes.contains(&hash) {
            return true;
        }
        if save {
            self.p2p_message_data_hashes.push(hash);
        }
        false
    }
    fn swarm_event(&mut self, event: SwarmEvent<OutEvent, HandlerErr>) {
        debug!("{:?}", event);
        match event {
            SwarmEvent::Dialing(_) => {}
            SwarmEvent::IncomingConnectionError { .. } => {}
            SwarmEvent::IncomingConnection { .. } => {}
            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                ..
            } => {
                Self::connection_established(self, peer_id, endpoint, num_established);
            }
            SwarmEvent::ConnectionClosed { endpoint, num_established, .. } => {
                Self::connection_closed(self, endpoint, num_established);
            }
            SwarmEvent::Behaviour(OutEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (_, multiaddr) in list {
                    if let Some(multiaddr) = multiaddr::filter_ip_port(&multiaddr) {
                        self.p2p_unknown.insert(multiaddr);
                    }
                }
            }
            SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                message, propagation_source, ..
            })) => {
                if self.filter(&message.data, false) {
                    return;
                }
                if let Err(err) = gossipsub::handler(self, message, propagation_source) {
                    debug!("{}", err)
                }
            }
            SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message { message, peer })) => match message {
                RequestResponseMessage::Request { request, channel, .. } => {
                    if let Err(err) = request_response::request_handler(self, peer, request, channel) {
                        debug!("{}", err)
                    }
                }
                RequestResponseMessage::Response { response, .. } => {
                    if let Err(err) = request_response::response_handler(self, peer, response) {
                        debug!("{}", err)
                    }
                }
            },
            SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::InboundFailure { .. })) => {}
            SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::OutboundFailure { .. })) => {}
            SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::ResponseSent { .. })) => {}
            _ => {}
        }
    }
    pub async fn start(&mut self) {
        self.blockchain.load();
        info!(
            "Blockchain height is {}",
            if let Some(main) = self.blockchain.tree.main() {
                main.1.to_string().yellow()
            } else {
                "0".red()
            }
        );
        info!("Latest block seen {}", self.last_seen().yellow());
        let multiaddr: Multiaddr = self.p2p_host.parse().unwrap();
        self.p2p_swarm.listen_on(multiaddr.clone()).unwrap();
        info!("Swarm is listening on {}", multiaddr.to_string().magenta());
        let listener = TcpListener::bind(&self.bind_api).await.unwrap();
        info!(
            "API is listening on {}{}",
            "http://".cyan(),
            listener.local_addr().unwrap().to_string().magenta()
        );
        let mut interval = tokio::time::interval(Duration::from_micros(util::micros_per_tick(self.tps)));
        loop {
            tokio::select! {
                instant = interval.tick() => heartbeat::handler(self, instant),
                event = self.p2p_swarm.select_next_some() => self.swarm_event(event),
                res = listener.accept() => match res {
                    Ok((stream, socket_addr)) => {
                        match http::handler(stream, self).await {
                            Ok((bytes, first)) => info!("{} {} {} {}", "API".cyan(), socket_addr.to_string().magenta(), bytes.to_string().yellow(), first),
                            Err(err) => error!("{} {} {}", "API".cyan(), socket_addr.to_string().magenta(), err)
                        }
                    }
                    Err(err) => error!("{} {}", "API".cyan(), err)
                }
            }
        }
    }
    fn connection_established(node: &mut Node, peer_id: PeerId, endpoint: ConnectedPoint, num_established: NonZeroU32) {
        let mut save = |multiaddr: Multiaddr| {
            info!(
                "Connection {} {} {}",
                "established".green(),
                multiaddr.to_string().magenta(),
                num_established.to_string().yellow()
            );
            let addr = multiaddr::addr(&multiaddr).expect("multiaddr to include ip");
            if node.p2p_ratelimit.is_ratelimited(&node.p2p_ratelimit.get(&addr).1) {
                warn!("Ratelimited {}", multiaddr.to_string().magenta());
                let _ = node.p2p_swarm.disconnect_peer_id(peer_id);
            }
            node.p2p_known.insert(multiaddr.clone());
            let _ = db::peer::put(&multiaddr.to_string(), &node.blockchain.db);
            if let Some(previous_peer_id) = node
                .p2p_connections
                .insert(multiaddr::filter_ip(&multiaddr).expect("multiaddr to include ip"), peer_id)
            {
                if previous_peer_id != peer_id {
                    let _ = node.p2p_swarm.disconnect_peer_id(previous_peer_id);
                }
            }
        };
        if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
            if let Some(multiaddr) = multiaddr::filter_ip_port(&address) {
                save(multiaddr);
            }
        }
        if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
            if let Some(multiaddr) = multiaddr::filter_ip(&send_back_addr) {
                save(multiaddr);
            }
        }
    }
    fn connection_closed(node: &mut Node, endpoint: ConnectedPoint, num_established: u32) {
        let mut save = |multiaddr: Multiaddr| {
            info!(
                "Connection {} {} {}",
                "closed".red(),
                multiaddr.to_string().magenta(),
                num_established.to_string().yellow()
            );
            node.p2p_connections.remove(&multiaddr);
            let _ = node.p2p_swarm.dial(multiaddr);
        };
        if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
            if let Some(multiaddr) = multiaddr::filter_ip_port(&address) {
                save(multiaddr);
            }
        }
        if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
            if let Some(multiaddr) = multiaddr::filter_ip(&send_back_addr) {
                save(multiaddr);
            }
        }
    }
    pub fn gossipsub_has_mesh_peers(&mut self, topic: &str) -> bool {
        self.p2p_swarm.behaviour().gossipsub.mesh_peers(&TopicHash::from_raw(topic)).count() != 0
    }
    pub fn gossipsub_publish(&mut self, topic: &str, data: Vec<u8>) {
        self.filter(&data, true);
        if let Err(err) = self.p2p_swarm.behaviour_mut().gossipsub.publish(IdentTopic::new(topic), data) {
            error!("{}", err);
        }
    }
    pub fn last_seen(&self) -> String {
        if self.blockchain.states.dynamic.latest_block.timestamp == 0 {
            return "never".to_string();
        }
        let timestamp = self.blockchain.states.dynamic.latest_block.timestamp;
        let diff = util::timestamp().saturating_sub(timestamp);
        let now = "just now";
        let mut string = util::duration_to_string(diff, now);
        if string != now {
            string.push_str(" ago");
        }
        string
    }
    pub fn sync_status(&self) -> String {
        let completed = "completed";
        if self.blockchain.sync.completed {
            return completed.to_string();
        }
        if !self.blockchain.sync.downloading() {
            return "waiting to start".to_string();
        }
        let timestamp = self.blockchain.states.dynamic.latest_block.timestamp;
        let mut diff = util::timestamp().saturating_sub(timestamp) as f32;
        diff /= BLOCK_TIME_MIN as f32;
        diff /= self.blockchain.sync.bps;
        let mut string = util::duration_to_string(diff as u32, completed);
        if string != completed {
            string.push_str(" remaining");
        }
        string
    }
    pub fn uptime(&self) -> String {
        let seconds = (self.heartbeats as f64 / self.tps) as u32;
        util::duration_to_string(seconds, "0")
    }
}
