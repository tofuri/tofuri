use libp2p::{
    autonat,
    gossipsub::{Gossipsub, GossipsubConfigBuilder, GossipsubEvent, MessageAuthenticity},
    identify, identity,
    mdns::{MdnsConfig, MdnsEvent, TokioMdns},
    ping,
    swarm::keep_alive,
    NetworkBehaviour,
};
use pea_core::constants::PROTOCOL_VERSION;
use std::error::Error;
use void::Void;
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct Behaviour {
    pub mdns: TokioMdns,
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    pub gossipsub: Gossipsub,
    pub autonat: autonat::Behaviour,
    pub keep_alive: keep_alive::Behaviour,
}
impl Behaviour {
    pub async fn new(local_key: identity::Keypair) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            mdns: TokioMdns::new(MdnsConfig::default())?,
            ping: ping::Behaviour::new(ping::Config::new()),
            identify: identify::Behaviour::new(identify::Config::new(PROTOCOL_VERSION.to_string(), local_key.public())),
            gossipsub: Gossipsub::new(MessageAuthenticity::Signed(local_key.clone()), GossipsubConfigBuilder::default().build()?)?,
            autonat: autonat::Behaviour::new(local_key.public().to_peer_id(), autonat::Config::default()),
            keep_alive: keep_alive::Behaviour::default(),
        })
    }
}
#[derive(Debug)]
pub enum OutEvent {
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
    Ping(ping::Event),
    Identify(identify::Event),
    Autonat(autonat::Event),
    Void(Void),
}
impl From<MdnsEvent> for OutEvent {
    fn from(v: MdnsEvent) -> Self {
        Self::Mdns(v)
    }
}
impl From<GossipsubEvent> for OutEvent {
    fn from(v: GossipsubEvent) -> Self {
        Self::Gossipsub(v)
    }
}
impl From<ping::Event> for OutEvent {
    fn from(v: ping::Event) -> Self {
        Self::Ping(v)
    }
}
impl From<identify::Event> for OutEvent {
    fn from(v: identify::Event) -> Self {
        Self::Identify(v)
    }
}
impl From<autonat::Event> for OutEvent {
    fn from(v: autonat::Event) -> Self {
        Self::Autonat(v)
    }
}
impl From<Void> for OutEvent {
    fn from(v: Void) -> Self {
        Self::Void(v)
    }
}
