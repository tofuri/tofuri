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
#[derive(Debug)]
pub enum Error {
    PublishError(PublishError),
    Behaviour(behaviour::Error),
    SubscriptionError(SubscriptionError),
    Filter,
}
pub struct P2p {
    pub swarm: Swarm<Behaviour>,
    pub connections: HashMap<PeerId, IpAddr>,
    pub timeouts: HashMap<IpAddr, u32>,
    pub requests: HashMap<IpAddr, usize>,
    pub unknown: HashSet<IpAddr>,
    pub known: HashSet<IpAddr>,
}
impl P2p {
    pub async fn new(max_established: Option<u32>, timeout: u64, known: HashSet<IpAddr>) -> Result<P2p, Error> {
        Ok(P2p {
            swarm: swarm(max_established, timeout).await?,
            connections: HashMap::new(),
            timeouts: HashMap::new(),
            requests: HashMap::new(),
            unknown: HashSet::new(),
            known,
        })
    }
    pub fn timeout(&mut self, peer_id: &PeerId) {
        let ip_addr = self.connections.get(peer_id).unwrap();
        self.timeouts.insert(*ip_addr, tofuri_util::timestamp());
    }
    pub fn has_timeout(&self, peer_id: &PeerId) -> bool {
        let ip_addr = self.connections.get(peer_id).unwrap();
        let timestamp = self.timeouts.get(ip_addr).unwrap_or(&0);
        tofuri_util::timestamp() - timestamp < P2P_TIMEOUT
    }
    pub fn request(&mut self, peer_id: &PeerId) -> bool {
        let ip_addr = self.connections.get(peer_id).unwrap();
        let mut requests = *self.requests.get(ip_addr).unwrap_or(&0);
        requests += 1;
        self.requests.insert(*ip_addr, requests);
        if requests > P2P_REQUESTS {
            self.timeout(peer_id);
        }
        self.has_timeout(peer_id)
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
