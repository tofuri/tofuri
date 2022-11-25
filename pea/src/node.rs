use crate::{
    behaviour::{Behaviour, OutEvent},
    blockchain::Blockchain,
    gossipsub, heartbeat, http, multiaddr,
};
use colored::*;
use futures::{FutureExt, StreamExt};
use libp2p::{
    core::{connection::ConnectedPoint, either::EitherError, upgrade},
    gossipsub::{error::GossipsubHandlerError, GossipsubEvent, IdentTopic},
    identity,
    mdns::MdnsEvent,
    mplex, noise,
    ping::Failure,
    swarm::{ConnectionHandlerUpgrErr, ConnectionLimits, SwarmBuilder, SwarmEvent},
    tcp::TokioTcpConfig,
    Multiaddr, PeerId, Swarm, Transport,
};
use log::{debug, error, info};
use pea_address as address;
use pea_core::{types, util};
use pea_db as db;
use pea_wallet::Wallet;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    num::NonZeroU32,
};
use tempdir::TempDir;
use tokio::net::TcpListener;
type HandlerErr =
    EitherError<EitherError<EitherError<EitherError<void::Void, Failure>, std::io::Error>, GossipsubHandlerError>, ConnectionHandlerUpgrErr<std::io::Error>>;
pub struct Options<'a> {
    pub tempdb: bool,
    pub tempkey: bool,
    pub genesis: bool,
    pub trust: usize,
    pub pending: usize,
    pub max_established: Option<u32>,
    pub tps: f64,
    pub wallet: &'a str,
    pub passphrase: &'a str,
    pub peer: &'a str,
    pub bind_api: String,
    pub host: String,
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
    pub genesis: bool,
}
impl Node {
    pub async fn new(options: Options<'_>) -> Node {
        let wallet = Node::wallet(options.tempkey, options.wallet, options.passphrase);
        info!("PubKey is {}", address::public::encode(&wallet.key.public_key_bytes()).green());
        let db = Node::db(options.tempdb);
        let blockchain = Blockchain::new(db, wallet.key, options.trust, options.pending);
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
            genesis: options.genesis,
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
        let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&local_key)
            .expect("Signing libp2p-noise static DH keypair failed.");
        let transport = TokioTcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(mplex::MplexConfig::new())
            .boxed();
        let mut behaviour = Behaviour::new(local_key).await?;
        for ident_topic in [
            IdentTopic::new("block"),
            IdentTopic::new("block sync"),
            IdentTopic::new("stake"),
            IdentTopic::new("transaction"),
            IdentTopic::new("multiaddr"),
        ]
        .iter()
        {
            behaviour.gossipsub.subscribe(ident_topic)?;
        }
        let mut limits = ConnectionLimits::default();
        limits = limits.with_max_established_per_peer(Some(2));
        limits = limits.with_max_established(max_established);
        Ok(SwarmBuilder::new(transport, behaviour, local_peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
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
    fn handle_event(&mut self, event: SwarmEvent<OutEvent, HandlerErr>) {
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
        let multiaddr: Multiaddr = self.host.parse().unwrap();
        self.swarm.listen_on(multiaddr.clone()).unwrap();
        info!("Swarm is listening on {}", multiaddr.to_string().magenta());
        if !self.bind_api.is_empty() {
            let listener = TcpListener::bind(&self.bind_api).await.unwrap();
            info!(
                "API is listening on {}{}",
                "http://".cyan(),
                listener.local_addr().unwrap().to_string().magenta()
            );
            loop {
                tokio::select! {
                    Ok(stream) = http::next(&listener).fuse() => if let Err(err) = http::handler(stream, self).await {
                        error!("{}", err);
                    },
                    _ = heartbeat::next(self.tps).fuse() => heartbeat::handler(self),
                    event = self.swarm.select_next_some() => self.handle_event(event),
                }
            }
        } else {
            info!("API is {}", "disabled".red());
            loop {
                tokio::select! {
                    _ = heartbeat::next(self.tps).fuse() => heartbeat::handler(self),
                    event = self.swarm.select_next_some() => self.handle_event(event),
                }
            }
        };
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
}