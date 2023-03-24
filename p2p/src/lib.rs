pub mod behaviour;
pub mod multiaddr;
use behaviour::Behaviour;
use libp2p::core::upgrade;
use libp2p::gossipsub::error::PublishError;
use libp2p::gossipsub::error::SubscriptionError;
use libp2p::gossipsub::IdentTopic;
use libp2p::gossipsub::TopicHash;
use libp2p::identity;
use libp2p::mplex;
use libp2p::noise;
use libp2p::swarm::ConnectionLimits;
use libp2p::swarm::SwarmBuilder;
use libp2p::tcp;
use libp2p::PeerId;
use libp2p::Swarm;
use libp2p::Transport;
use std::collections::HashMap;
use std::collections::HashSet;
use std::net::IpAddr;
use std::time::Duration;
use tofuri_core::*;
use tracing::log::warn;
#[derive(Debug)]
pub enum Error {
    PublishError(PublishError),
    Behaviour(behaviour::Error),
    SubscriptionError(SubscriptionError),
}
pub struct P2p {
    pub swarm: Swarm<Behaviour>,
    pub connections: HashMap<PeerId, IpAddr>,
    pub connections_unknown: HashSet<IpAddr>,
    pub connections_known: HashSet<IpAddr>,
    pub request_timeouts: HashMap<IpAddr, u32>,
    pub request_counter: HashMap<IpAddr, usize>,
    pub gossipsub_message_counter_blocks: HashMap<IpAddr, usize>,
    pub gossipsub_message_counter_transactions: HashMap<IpAddr, usize>,
    pub gossipsub_message_counter_stakes: HashMap<IpAddr, usize>,
    pub gossipsub_message_counter_peers: HashMap<IpAddr, usize>,
}
impl P2p {
    pub async fn new(max_established: Option<u32>, timeout: u64, known: HashSet<IpAddr>) -> Result<P2p, Error> {
        Ok(P2p {
            swarm: swarm(max_established, timeout).await?,
            connections: HashMap::new(),
            connections_unknown: HashSet::new(),
            connections_known: known,
            request_timeouts: HashMap::new(),
            request_counter: HashMap::new(),
            gossipsub_message_counter_blocks: HashMap::new(),
            gossipsub_message_counter_transactions: HashMap::new(),
            gossipsub_message_counter_stakes: HashMap::new(),
            gossipsub_message_counter_peers: HashMap::new(),
        })
    }
    fn get_ip_addr(&self, peer_id: &PeerId) -> Option<IpAddr> {
        if let Some(ip_addr) = self.connections.get(peer_id).cloned() {
            Some(ip_addr)
        } else {
            warn!("Peer {} not found in connections", peer_id);
            None
        }
    }
    pub fn request_timeout(&mut self, peer_id: &PeerId) {
        let opt = self.get_ip_addr(peer_id);
        if opt.is_none() {
            return;
        }
        let ip_addr = opt.unwrap();
        self.request_timeouts.insert(ip_addr, tofuri_util::timestamp());
    }
    pub fn request_counter(&mut self, peer_id: &PeerId) -> bool {
        let opt = self.get_ip_addr(peer_id);
        if opt.is_none() {
            return true;
        }
        let ip_addr = opt.unwrap();
        let mut requests = *self.request_counter.get(&ip_addr).unwrap_or(&0);
        requests += 1;
        self.request_counter.insert(ip_addr, requests);
        if requests > P2P_REQUESTS {
            self.request_timeout(peer_id);
        }
        let timestamp = self.request_timeouts.get(&ip_addr).unwrap_or(&0);
        tofuri_util::timestamp() - timestamp < P2P_TIMEOUT
    }
    fn gossipsub_message_counter(connections: &HashMap<PeerId, IpAddr>, map: &mut HashMap<IpAddr, usize>, limit: usize, peer_id: &PeerId) -> bool {
        let option_ip_addr = connections.get(peer_id);
        if option_ip_addr.is_none() {
            return true;
        }
        let ip_addr = option_ip_addr.unwrap();
        let mut counter = *map.get(&ip_addr).unwrap_or(&0);
        counter += 1;
        map.insert(*ip_addr, counter);
        counter > limit
    }
    pub fn gossipsub_message_counter_blocks(&mut self, peer_id: &PeerId) -> bool {
        P2p::gossipsub_message_counter(&self.connections, &mut self.gossipsub_message_counter_blocks, P2P_BLOCKS, peer_id)
    }
    pub fn gossipsub_message_counter_transactions(&mut self, peer_id: &PeerId) -> bool {
        P2p::gossipsub_message_counter(&self.connections, &mut self.gossipsub_message_counter_transactions, P2P_TRANSACTIONS, peer_id)
    }
    pub fn gossipsub_message_counter_stakes(&mut self, peer_id: &PeerId) -> bool {
        P2p::gossipsub_message_counter(&self.connections, &mut self.gossipsub_message_counter_stakes, P2P_STAKES, peer_id)
    }
    pub fn gossipsub_message_counter_peers(&mut self, peer_id: &PeerId) -> bool {
        P2p::gossipsub_message_counter(&self.connections, &mut self.gossipsub_message_counter_peers, P2P_PEERS, peer_id)
    }
    fn gossipsub_has_mesh_peers(&self, topic: &str) -> bool {
        self.swarm.behaviour().gossipsub.mesh_peers(&TopicHash::from_raw(topic)).count() != 0
    }
    pub fn gossipsub_publish(&mut self, topic: &str, data: Vec<u8>) -> Result<(), Error> {
        if !self.gossipsub_has_mesh_peers(topic) {
            return Ok(());
        }
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(IdentTopic::new(topic), data)
            .map_err(Error::PublishError)?;
        Ok(())
    }
}
async fn swarm(max_established: Option<u32>, timeout: u64) -> Result<Swarm<Behaviour>, Error> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseAuthenticated::xx(&local_key).expect("Signing libp2p-noise static DH keypair failed."))
        .multiplex(mplex::MplexConfig::new())
        .timeout(Duration::from_millis(timeout))
        .boxed();
    let mut behaviour = Behaviour::new(local_key).await.map_err(Error::Behaviour)?;
    for ident_topic in [
        IdentTopic::new("block"),
        IdentTopic::new("stake"),
        IdentTopic::new("transaction"),
        IdentTopic::new("peers"),
    ]
    .iter()
    {
        behaviour.gossipsub.subscribe(ident_topic).map_err(Error::SubscriptionError)?;
    }
    let mut limits = ConnectionLimits::default();
    limits = limits.with_max_established_per_peer(Some(1));
    limits = limits.with_max_established(max_established);
    Ok(SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id)
        .connection_limits(limits)
        .build())
}
