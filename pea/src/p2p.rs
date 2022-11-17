use libp2p::{
    autonat,
    core::upgrade,
    gossipsub::{Gossipsub, GossipsubConfigBuilder, GossipsubEvent, IdentTopic, MessageAuthenticity},
    identify, identity,
    mdns::{MdnsConfig, MdnsEvent, TokioMdns},
    mplex, noise, ping,
    swarm::SwarmBuilder,
    tcp::TokioTcpTransport,
    NetworkBehaviour, PeerId, Swarm, Transport,
};
use pea_core::constants::PROTOCOL_VERSION;
use std::error::Error;
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "MyBehaviourEvent")]
pub struct MyBehaviour {
    pub mdns: TokioMdns,
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    pub gossipsub: Gossipsub,
    pub autonat: autonat::Behaviour,
}
impl MyBehaviour {
    async fn new(local_key: identity::Keypair) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            mdns: TokioMdns::new(MdnsConfig::default())?,
            ping: ping::Behaviour::new(ping::Config::new()),
            identify: identify::Behaviour::new(identify::Config::new(PROTOCOL_VERSION.to_string(), local_key.public())),
            gossipsub: Gossipsub::new(MessageAuthenticity::Signed(local_key.clone()), GossipsubConfigBuilder::default().build()?)?,
            autonat: autonat::Behaviour::new(local_key.public().to_peer_id(), autonat::Config::default()),
        })
    }
}
#[derive(Debug)]
pub enum MyBehaviourEvent {
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
    Ping(ping::Event),
    Identify(identify::Event),
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
impl From<ping::Event> for MyBehaviourEvent {
    fn from(v: ping::Event) -> Self {
        Self::Ping(v)
    }
}
impl From<identify::Event> for MyBehaviourEvent {
    fn from(v: identify::Event) -> Self {
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
    let transport = TokioTcpTransport::default()
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseAuthenticated::xx(&local_key).expect("Signing libp2p-noise static DH keypair failed."))
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
