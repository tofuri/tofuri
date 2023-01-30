pub mod behaviour;
pub mod multiaddr;
pub mod ratelimit;
use behaviour::Behaviour;
use libp2p::core::upgrade;
use libp2p::gossipsub::IdentTopic;
use libp2p::identity;
use libp2p::mplex;
use libp2p::noise;
use libp2p::swarm::ConnectionLimits;
use libp2p::swarm::SwarmBuilder;
use libp2p::tcp;
use libp2p::Multiaddr;
use libp2p::PeerId;
use libp2p::Swarm;
use libp2p::Transport;
use pea_core::*;
use ratelimit::Ratelimit;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::time::Duration;
pub struct P2p {
    pub swarm: Swarm<Behaviour>,
    pub message_data_hashes: Vec<Hash>,
    pub connections: HashMap<Multiaddr, PeerId>,
    pub ratelimit: Ratelimit,
    pub unknown: HashSet<Multiaddr>,
    pub known: HashSet<Multiaddr>,
    pub ban_offline: usize,
}
impl P2p {
    pub async fn new(max_established: Option<u32>, timeout: u64, known: HashSet<Multiaddr>, ban_offline: usize) -> Result<P2p, Box<dyn Error>> {
        Ok(P2p {
            swarm: swarm(max_established, timeout).await?,
            message_data_hashes: vec![],
            connections: HashMap::new(),
            ratelimit: Ratelimit::default(),
            unknown: HashSet::new(),
            known,
            ban_offline,
        })
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
