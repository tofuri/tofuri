use libp2p::{
    autonat,
    core::upgrade,
    gossipsub::{Gossipsub, GossipsubConfigBuilder, GossipsubEvent, IdentTopic, MessageAuthenticity},
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    identity,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    mplex, noise,
    ping::{self, Ping, PingEvent},
    swarm::SwarmBuilder,
    tcp::TokioTcpConfig,
    NetworkBehaviour, PeerId, Swarm, Transport,
};
use pea_core::constants::PROTOCOL_VERSION;
use std::error::Error;
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "MyBehaviourEvent")]
pub struct MyBehaviour {
    pub mdns: Mdns,
    pub ping: Ping,
    pub identify: Identify,
    pub gossipsub: Gossipsub,
    pub autonat: autonat::Behaviour,
}
impl MyBehaviour {
    async fn new(local_key: identity::Keypair) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            mdns: Mdns::new(MdnsConfig::default()).await?,
            ping: ping::Behaviour::new(ping::Config::new().with_keep_alive(true)),
            identify: Identify::new(IdentifyConfig::new(PROTOCOL_VERSION.to_string(), local_key.public())),
            gossipsub: Gossipsub::new(MessageAuthenticity::Signed(local_key.clone()), GossipsubConfigBuilder::default().build()?)?,
            autonat: autonat::Behaviour::new(local_key.public().to_peer_id(), autonat::Config::default()),
        })
    }
}
#[derive(Debug)]
pub enum MyBehaviourEvent {
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
    Ping(PingEvent),
    Identify(IdentifyEvent),
    Autonat(autonat::Event),
}
impl From<MdnsEvent> for MyBehaviourEvent {
    fn from(v: MdnsEvent) -> Self {
        Self::Mdns(v)
    }
}
impl From<GossipsubEvent> for MyBehaviourEvent {
    fn from(v: GossipsubEvent) -> Self {
        Self::Gossipsub(v)
    }
}
impl From<PingEvent> for MyBehaviourEvent {
    fn from(v: PingEvent) -> Self {
        Self::Ping(v)
    }
}
impl From<IdentifyEvent> for MyBehaviourEvent {
    fn from(v: IdentifyEvent) -> Self {
        Self::Identify(v)
    }
}
impl From<autonat::Event> for MyBehaviourEvent {
    fn from(v: autonat::Event) -> Self {
        Self::Autonat(v)
    }
}
pub async fn swarm() -> Result<Swarm<MyBehaviour>, Box<dyn Error>> {
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
    let mut behaviour = MyBehaviour::new(local_key).await?;
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
