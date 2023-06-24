pub mod behaviour;
pub mod multiaddr;
pub mod ratelimit;
pub mod swarm;
use behaviour::Behaviour;
use libp2p::core::upgrade;
use libp2p::gossipsub::IdentTopic;
use libp2p::gossipsub::PublishError;
use libp2p::gossipsub::SubscriptionError;
use libp2p::gossipsub::TopicHash;
use libp2p::identity;
use libp2p::noise;
use libp2p::swarm::SwarmBuilder;
use libp2p::tcp;
use libp2p::yamux;
use libp2p::PeerId;
use libp2p::Swarm;
use libp2p::Transport;
use ratelimit::Ratelimit;
use std::collections::HashMap;
use std::collections::HashSet;
use std::net::IpAddr;
use std::time::Duration;
use tracing::log::warn;
pub const MAX_TRANSMIT_SIZE: usize = 100_000;
pub const PROTOCOL_VERSION: &str = "tofuri/1.0.0";
pub const PROTOCOL_NAME: &str = "/sync/1";
pub const P2P_RATELIMIT_REQUEST_TIMEOUT: u32 = 3600;
pub const P2P_RATELIMIT_RESPONSE_TIMEOUT: u32 = 3600;
pub const P2P_RATELIMIT_REQUEST: usize = 60 + 1;
pub const P2P_RATELIMIT_RESPONSE: usize = 60 + 1;
pub const P2P_RATELIMIT_GOSSIPSUB_MESSAGE_BLOCK: usize = 1 + 1;
pub const P2P_RATELIMIT_GOSSIPSUB_MESSAGE_TRANSACTION: usize = 60 * 100;
pub const P2P_RATELIMIT_GOSSIPSUB_MESSAGE_STAKE: usize = 60 * 100;
pub const P2P_RATELIMIT_GOSSIPSUB_MESSAGE_PEERS: usize = 1 + 1;
pub const MAINNET_PORT: u16 = 2020;
pub const TESTNET_PORT: u16 = 3030;
#[derive(Debug)]
pub enum Error {
    PublishError(PublishError),
    Behaviour(behaviour::Error),
    SubscriptionError(SubscriptionError),
}
pub struct P2P {
    pub swarm: Swarm<Behaviour>,
    pub connections: HashMap<PeerId, IpAddr>,
    pub connections_unknown: HashSet<IpAddr>,
    pub connections_known: HashSet<IpAddr>,
    pub ratelimit: Ratelimit,
}
impl P2P {
    pub async fn new(
        max_established: Option<u32>,
        timeout: u64,
        connections_known: HashSet<IpAddr>,
    ) -> Result<P2P, Error> {
        let p2p = P2P {
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
        .authenticate(noise::Config::new(&local_key).unwrap())
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_millis(timeout))
        .boxed();
    let mut behaviour = Behaviour::new(local_key, max_established)
        .await
        .map_err(Error::Behaviour)?;
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
    let swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();
    Ok(swarm)
}
