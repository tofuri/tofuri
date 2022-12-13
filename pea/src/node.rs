use crate::{
    behaviour::{Behaviour, OutEvent},
    blockchain::Blockchain,
    gossipsub, heartbeat, http, multiaddr,
};
use colored::*;
use libp2p::{
    core::{connection::ConnectedPoint, either::EitherError, upgrade},
    futures::StreamExt,
    gossipsub::{error::GossipsubHandlerError, GossipsubEvent, IdentTopic, TopicHash},
    identity,
    mdns::MdnsEvent,
    mplex, noise,
    swarm::{ConnectionHandlerUpgrErr, ConnectionLimits, SwarmBuilder, SwarmEvent},
    tcp,
    tcp::GenTcpConfig,
    Multiaddr, PeerId, Swarm, Transport,
};
use log::{debug, error, info, warn};
use pea_address as address;
use pea_core::{constants::BLOCK_TIME_MIN, types, util};
use pea_db as db;
use pea_time::Time;
use pea_wallet::Wallet;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    num::NonZeroU32,
    time::Duration,
};
use tempdir::TempDir;
use tokio::net::TcpListener;
use void::Void;
type HandlerErr = EitherError<EitherError<EitherError<Void, std::io::Error>, GossipsubHandlerError>, ConnectionHandlerUpgrErr<std::io::Error>>;
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
}
pub struct Node {
    pub swarm: Swarm<Behaviour>,
    pub blockchain: Blockchain,
    pub message_data_hashes: Vec<types::Hash>,
    pub heartbeats: usize,
    pub lag: f64,
    pub tps: f64,
    pub unknown: HashSet<Multiaddr>,
    pub known: HashSet<Multiaddr>,
    pub connections: HashMap<Multiaddr, PeerId>,
    pub bind_api: String,
    pub host: String,
    pub mint: bool,
    pub ban_offline: usize,
    pub max_established: Option<u32>,
    pub tempdb: bool,
    pub tempkey: bool,
    pub time: Time,
    pub time_api: bool,
    pub dev: bool,
}
impl Node {
    pub async fn new(options: Options<'_>) -> Node {
        let wallet = Node::wallet(options.tempkey, options.wallet, options.passphrase);
        info!("PubKey is {}", address::public::encode(&wallet.key.public_key_bytes()).green());
        let db = Node::db(options.tempdb);
        let blockchain = Blockchain::new(db, wallet.key, options.trust, options.pending, options.time_delta);
        let swarm = Node::swarm(options.max_established).await.unwrap();
        let known = Node::known(&blockchain.db, options.peer);
        Node {
            swarm,
            blockchain,
            message_data_hashes: vec![],
            heartbeats: 0,
            lag: 0.0,
            tps: options.tps,
            unknown: HashSet::new(),
            known,
            connections: HashMap::new(),
            bind_api: options.bind_api,
            host: options.host,
            mint: options.mint,
            ban_offline: options.ban_offline,
            max_established: options.max_established,
            tempdb: options.tempdb,
            tempkey: options.tempkey,
            time: Time::new(),
            time_api: options.time_api,
            dev: options.dev,
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
    fn wallet(tempkey: bool, wallet: &str, passphrase: &str) -> Wallet {
        match tempkey {
            true => Wallet::new(),
            false => Wallet::import(wallet, passphrase).unwrap(),
        }
    }
    async fn swarm(max_established: Option<u32>) -> Result<Swarm<Behaviour>, Box<dyn Error>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Peer id {}", local_peer_id.to_string().cyan());
        let transport = tcp::TokioTcpTransport::new(GenTcpConfig::new())
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&local_key).expect("Signing libp2p-noise static DH keypair failed."))
            .multiplex(mplex::MplexConfig::new())
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
        Ok(SwarmBuilder::new(transport, behaviour, local_peer_id)
            .executor(Box::new(|fut| {
                tokio::task::spawn(fut);
            }))
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
        let hash = util::hash(data);
        if self.message_data_hashes.contains(&hash) {
            return true;
        }
        if save {
            self.message_data_hashes.push(hash);
        }
        false
    }
    fn swarm_event(&mut self, event: SwarmEvent<OutEvent, HandlerErr>) {
        debug!("{:?}", event);
        match event {
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
            SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                for (_, multiaddr) in list {
                    if let Some(multiaddr) = multiaddr::filter_ip_port(&multiaddr) {
                        self.unknown.insert(multiaddr);
                    }
                }
            }
            SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message { message, .. })) => {
                if self.filter(&message.data, false) {
                    return;
                }
                if let Err(err) = gossipsub::handler(self, message) {
                    debug!("{}", err)
                }
            }
            _ => {}
        }
    }
    pub async fn sync_time(&mut self) {
        if self.time.sync().await {
            info!(
                "Successfully adjusted for time difference. System clock is {} the world clock.",
                format!(
                    "{:?} {}",
                    Duration::from_micros(self.time.diff.abs() as u64),
                    if self.time.diff.is_negative() { "behind" } else { "ahead of" }
                )
                .to_string()
                .yellow()
            );
        } else {
            warn!("{}", "Failed to adjust for time difference!".red());
        }
    }
    pub async fn start(&mut self) {
        if self.time_api {
            self.sync_time().await;
        }
        self.blockchain.load();
        info!(
            "Blockchain height is {}",
            if let Some(main) = self.blockchain.tree.main() {
                main.1.to_string().yellow()
            } else {
                "0".red()
            }
        );
        info!("Latest block seen {}", self.last().yellow());
        let multiaddr: Multiaddr = self.host.parse().unwrap();
        self.swarm.listen_on(multiaddr.clone()).unwrap();
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
                event = self.swarm.select_next_some() => self.swarm_event(event),
                res = listener.accept() => match res {
                    Ok((stream, socket_addr)) => {
                        match http::handler(stream, self).await {
                            Ok(first) => info!("{} {} {}", "API".cyan(), socket_addr.to_string().magenta(), first),
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
            node.known.insert(multiaddr.clone());
            let _ = db::peer::put(&multiaddr.to_string(), &[], &node.blockchain.db);
            if let Some(previous_peer_id) = node
                .connections
                .insert(multiaddr::filter_ip(&multiaddr).expect("multiaddr to include ip"), peer_id)
            {
                if previous_peer_id != peer_id {
                    let _ = node.swarm.disconnect_peer_id(previous_peer_id);
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
            node.connections.remove(&multiaddr);
            let _ = node.swarm.dial(multiaddr);
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
        self.swarm.behaviour().gossipsub.mesh_peers(&TopicHash::from_raw(topic)).count() != 0
    }
    pub fn gossipsub_publish(&mut self, topic: &str, data: Vec<u8>) {
        self.filter(&data, true);
        if let Err(err) = self.swarm.behaviour_mut().gossipsub.publish(IdentTopic::new(topic), data) {
            error!("{}", err);
        }
    }
    pub fn last(&self) -> String {
        if self.blockchain.states.dynamic.latest_block.timestamp == 0 {
            return "never".to_string();
        }
        let timestamp = self.blockchain.states.dynamic.latest_block.timestamp;
        let diff = self.time.timestamp_secs().saturating_sub(timestamp);
        let now = "just now";
        let mut string = util::duration_to_string(diff, now);
        if string != now {
            string.push_str(" ago");
        }
        string
    }
    pub fn sync(&self) -> String {
        let completed = "completed";
        if self.blockchain.sync.completed {
            return completed.to_string();
        }
        if !self.blockchain.sync.downloading() {
            return "waiting to start".to_string();
        }
        let timestamp = self.blockchain.states.dynamic.latest_block.timestamp;
        let mut diff = self.time.timestamp_secs().saturating_sub(timestamp) as f32;
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
