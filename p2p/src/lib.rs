pub mod behaviour;
pub mod multiaddr;
pub mod ratelimit;
use behaviour::Behaviour;
use libp2p::core::upgrade;
use libp2p::gossipsub::IdentTopic;
use libp2p::gossipsub::PublishError;
use libp2p::gossipsub::SubscriptionError;
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
use ratelimit::Ratelimit;
use std::collections::HashMap;
use std::collections::HashSet;
use std::net::IpAddr;
use std::time::Duration;
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
    pub ratelimit: Ratelimit,
}
impl P2p {
    pub async fn new(
        max_established: Option<u32>,
        timeout: u64,
        connections_known: HashSet<IpAddr>,
    ) -> Result<P2p, Error> {
        let p2p = P2p {
            swarm: swarm(max_established, timeout).await?,
            connections: HashMap::new(),
            connections_unknown: HashSet::new(),
            connections_known,
            ratelimit: Ratelimit::default(),
        };
        Ok(p2p)
    }
    pub fn vec_ip_addr(&self, peer_ids: &[PeerId]) -> Vec<IpAddr> {
        let mut vec = vec![];
        for peer_id in peer_ids {
            if let Some(ip_addr) = self.connections.get(peer_id).cloned() {
                if vec.contains(&ip_addr) {
                    continue;
                }
                vec.push(ip_addr);
            } else {
                warn!("Peer {} not found in connections", peer_id);
            }
        }
        vec
    }
    fn gossipsub_has_mesh_peers(&self, topic: &str) -> bool {
        self.swarm
            .behaviour()
            .gossipsub
            .mesh_peers(&TopicHash::from_raw(topic))
            .count()
            != 0
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
        .authenticate(
            noise::NoiseAuthenticated::xx(&local_key)
                .expect("Signing libp2p-noise static DH keypair failed."),
        )
        .multiplex(mplex::MplexConfig::new())
        .timeout(Duration::from_millis(timeout))
        .boxed();
    let mut behaviour = Behaviour::new(local_key).await.map_err(Error::Behaviour)?;
    let topics = [
        IdentTopic::new("block"),
        IdentTopic::new("stake"),
        IdentTopic::new("transaction"),
        IdentTopic::new("peers"),
    ];
    for topic in topics.iter() {
        behaviour
            .gossipsub
            .subscribe(topic)
            .map_err(Error::SubscriptionError)?;
    }
    let mut limits = ConnectionLimits::default();
    limits = limits.with_max_established_per_peer(Some(1));
    limits = limits.with_max_established(max_established);
    let swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id)
        .connection_limits(limits)
        .build();
    Ok(swarm)
}
