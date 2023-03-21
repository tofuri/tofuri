pub mod behaviour;
pub mod multiaddr;
pub mod ratelimit;
use behaviour::Behaviour;
use libp2p::core::upgrade;
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
use ratelimit::Endpoint;
use ratelimit::Ratelimit;
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::net::IpAddr;
use std::time::Duration;
use tofuri_core::*;
pub struct P2p {
    pub swarm: Swarm<Behaviour>,
    pub filter: HashSet<Hash>,
    pub connections: HashMap<PeerId, IpAddr>,
    pub ratelimit: Ratelimit,
    pub unknown: HashSet<IpAddr>,
    pub known: HashSet<IpAddr>,
}
impl P2p {
    pub async fn new(max_established: Option<u32>, timeout: u64, known: HashSet<IpAddr>) -> Result<P2p, Box<dyn Error>> {
        Ok(P2p {
            swarm: swarm(max_established, timeout).await?,
            filter: HashSet::new(),
            connections: HashMap::new(),
            ratelimit: Ratelimit::default(),
            unknown: HashSet::new(),
            known,
        })
    }
    pub fn ratelimit(&mut self, peer_id: PeerId, endpoint: Endpoint) -> Result<(), Box<dyn Error>> {
        let ip_addr = self.connections.get(&peer_id).unwrap();
        if self.ratelimit.add(*ip_addr, endpoint) {
            let _ = self.swarm.disconnect_peer_id(peer_id);
            return Err("ratelimited".into());
        }
        Ok(())
    }
    pub fn filter(&mut self, data: &[u8]) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(data);
        !self.filter.insert(hasher.finalize().into())
    }
    fn gossipsub_has_mesh_peers(&self, topic: &str) -> bool {
        self.swarm.behaviour().gossipsub.mesh_peers(&TopicHash::from_raw(topic)).count() != 0
    }
    pub fn gossipsub_publish(&mut self, topic: &str, data: Vec<u8>) -> Result<(), Box<dyn Error>> {
        if !self.gossipsub_has_mesh_peers(topic) {
            return Ok(());
        }
        if self.filter(&data) {
            return Err(format!("gossipsub_publish filter {topic}").into());
        }
        if let Err(err) = self.swarm.behaviour_mut().gossipsub.publish(IdentTopic::new(topic), data) {
            return Err(err.into());
        }
        Ok(())
    }
}
async fn swarm(max_established: Option<u32>, timeout: u64) -> Result<Swarm<Behaviour>, Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseAuthenticated::xx(&local_key).expect("Signing libp2p-noise static DH keypair failed."))
        .multiplex(mplex::MplexConfig::new())
        .timeout(Duration::from_millis(timeout))
        .boxed();
    let mut behaviour = Behaviour::new(local_key).await?;
    for ident_topic in [
        IdentTopic::new("block"),
        IdentTopic::new("stake"),
        IdentTopic::new("transaction"),
        IdentTopic::new("ip_addr"),
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
