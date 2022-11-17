use crate::{
    behaviour::{Behaviour, Event},
    blockchain::Blockchain,
    gossipsub, heartbeat, http,
};
use colored::*;
use futures::{FutureExt, StreamExt};
use libp2p::{
    core::{connection::ConnectedPoint, either::EitherError, upgrade},
    gossipsub::{error::GossipsubHandlerError, GossipsubEvent, IdentTopic},
    identity,
    mdns::MdnsEvent,
    mplex,
    multiaddr::Protocol,
    noise,
    ping::Failure,
    swarm::{ConnectionHandlerUpgrErr, SwarmBuilder, SwarmEvent},
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
};
use tempdir::TempDir;
use tokio::net::TcpListener;
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
    pub bind_http_api: String,
}
impl Node {
    pub async fn new(tempdb: bool, tempkey: bool, trust: usize, pending: usize, tps: f64, wallet: &str, passphrase: &str, peer: &str, bind_http_api: String) -> Node {
        let db = Node::db(tempdb);
        let wallet = Node::wallet(tempkey, wallet, passphrase);
        info!("{} {}", "PubKey".cyan(), address::public::encode(&wallet.key.public_key_bytes()).green());
        let blockchain = Blockchain::new(db, wallet.key, trust, pending);
        let swarm = Node::swarm().await.unwrap();
        let known = Node::known(&blockchain.db, peer);
        Node {
            swarm,
            blockchain,
            message_data_hashes: vec![],
            heartbeats: 0,
            lag: 0.0,
            tps,
            unknown: HashSet::new(),
            known,
            connections: HashMap::new(),
            bind_http_api,
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
    async fn swarm() -> Result<Swarm<Behaviour>, Box<dyn Error>> {
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
        Ok(SwarmBuilder::new(transport, behaviour, local_peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build())
    }
    fn known(db: &DBWithThreadMode<SingleThreaded>, peer: &str) -> HashSet<Multiaddr> {
        let mut known = HashSet::new();
        if let Some(multiaddr) = Node::multiaddr_ip_port(peer.parse::<Multiaddr>().unwrap()) {
            known.insert(multiaddr);
        }
        let peers = db::peer::get_all(db);
        for peer in peers {
            if let Some(multiaddr) = Node::multiaddr_ip_port(peer.parse::<Multiaddr>().unwrap()) {
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
    fn handle_event(
        &mut self,
        event: SwarmEvent<Event, EitherError<EitherError<EitherError<EitherError<void::Void, Failure>, std::io::Error>, GossipsubHandlerError>, ConnectionHandlerUpgrErr<std::io::Error>>>,
    ) {
        debug!("{:?}", event);
        match event {
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                Self::connection_established(self, peer_id, endpoint);
            }
            SwarmEvent::ConnectionClosed { endpoint, .. } => {
                Self::connection_closed(self, endpoint);
            }
            SwarmEvent::Behaviour(Event::Mdns(MdnsEvent::Discovered(list))) => {
                for (_, multiaddr) in list {
                    if let Some(multiaddr) = Self::multiaddr_ip_port(multiaddr) {
                        self.unknown.insert(multiaddr);
                    }
                }
            }
            SwarmEvent::Behaviour(Event::Gossipsub(GossipsubEvent::Message { message, .. })) => {
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
    pub async fn start(&mut self, host: &str) {
        self.blockchain.load();
        self.swarm.listen_on(host.parse().unwrap()).unwrap();
        let tcp_listener_http_api = if !self.bind_http_api.is_empty() {
            Some(TcpListener::bind(&self.bind_http_api).await.unwrap())
        } else {
            None
        };
        if let Some(listener) = tcp_listener_http_api {
            info!("{} {} http://{}", "Enabled".green(), "HTTP API".cyan(), listener.local_addr().unwrap().to_string().green());
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
            info!("{} {}", "HTTP API".cyan(), "Disabled".red());
            loop {
                tokio::select! {
                    _ = heartbeat::next(self.tps).fuse() => heartbeat::handler(self),
                    event = self.swarm.select_next_some() => self.handle_event(event),
                }
            }
        }
    }
    fn connection_established(node: &mut Node, peer_id: PeerId, endpoint: ConnectedPoint) {
        let mut save = |multiaddr: Multiaddr| {
            node.known.insert(multiaddr.clone());
            let _ = db::peer::put(&multiaddr.to_string(), &[], &node.blockchain.db);
            if let Some(previous_peer_id) = node.connections.insert(multiaddr, peer_id) {
                if previous_peer_id != peer_id {
                    let _ = node.swarm.disconnect_peer_id(previous_peer_id);
                }
            }
        };
        if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
            if let Some(multiaddr) = Node::multiaddr_ip_port(address) {
                save(multiaddr);
            }
        }
        if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
            if let Some(multiaddr) = Node::multiaddr_ip(send_back_addr) {
                save(multiaddr);
            }
        }
    }
    fn connection_closed(node: &mut Node, endpoint: ConnectedPoint) {
        let mut save = |multiaddr: Multiaddr| {
            node.connections.remove(&multiaddr);
            let _ = node.swarm.dial(multiaddr);
        };
        if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
            if let Some(multiaddr) = Node::multiaddr_ip_port(address) {
                save(multiaddr);
            }
        }
        if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
            if let Some(multiaddr) = Node::multiaddr_ip(send_back_addr) {
                save(multiaddr);
            }
        }
    }
    pub fn multiaddr_ip(multiaddr: Multiaddr) -> Option<Multiaddr> {
        let components = multiaddr.iter().collect::<Vec<_>>();
        let mut multiaddr: Multiaddr = "".parse().unwrap();
        match components.get(0) {
            Some(Protocol::Ip4(ip)) => multiaddr.push(Protocol::Ip4(*ip)),
            Some(Protocol::Ip6(ip)) => multiaddr.push(Protocol::Ip6(*ip)),
            _ => return None,
        };
        Some(multiaddr)
    }
    pub fn multiaddr_ip_port(multiaddr: Multiaddr) -> Option<Multiaddr> {
        let components = multiaddr.iter().collect::<Vec<_>>();
        let mut multiaddr: Multiaddr = "".parse().unwrap();
        match components.get(0) {
            Some(Protocol::Ip4(ip)) => multiaddr.push(Protocol::Ip4(*ip)),
            Some(Protocol::Ip6(ip)) => multiaddr.push(Protocol::Ip6(*ip)),
            _ => return None,
        };
        match components.get(1) {
            Some(Protocol::Tcp(port)) => {
                if port == &9333_u16 {
                    return Some(multiaddr);
                }
                multiaddr.push(Protocol::Tcp(*port))
            }
            _ => return Some(multiaddr),
        };
        Some(multiaddr)
    }
    pub fn multiaddr_has_port(multiaddr: &Multiaddr) -> bool {
        let components = multiaddr.iter().collect::<Vec<_>>();
        match components.get(1) {
            Some(Protocol::Tcp(_)) => true,
            _ => false,
        }
    }
}
