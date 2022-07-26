use super::{constants::PROTOCOL_VERSION, util::print, validator::Validator};
use colored::*;
use libp2p::{
    autonat,
    floodsub::{Floodsub, FloodsubEvent},
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubEvent, IdentTopic, MessageAuthenticity,
    },
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    identity,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    ping::{self, Ping, PingEvent},
    relay::v2::relay::{self, Relay},
    swarm::{NetworkBehaviourEventProcess, Swarm},
    NetworkBehaviour, PeerId,
};
use std::{error::Error, time::Duration};
#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct MyBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    pub ping: Ping,
    pub identify: Identify,
    pub gossipsub: Gossipsub,
    pub autonat: autonat::Behaviour,
    pub relay: Relay,
    #[behaviour(ignore)]
    pub validator: Validator,
}
impl MyBehaviour {
    async fn new(
        local_key: identity::Keypair,
        validator: Validator,
    ) -> Result<Self, Box<dyn Error>> {
        let local_public_key = local_key.public();
        let local_peer_id = PeerId::from(local_public_key.clone());
        Ok(Self {
            floodsub: Floodsub::new(PeerId::from(local_public_key.clone())),
            mdns: Mdns::new(MdnsConfig::default()).await?,
            ping: ping::Behaviour::new(ping::Config::new().with_keep_alive(true)),
            identify: Identify::new(IdentifyConfig::new(
                PROTOCOL_VERSION.to_string(),
                local_public_key.clone(),
            )),
            gossipsub: Gossipsub::new(
                MessageAuthenticity::Signed(local_key.clone()),
                GossipsubConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                    .build()?,
            )?,
            autonat: autonat::Behaviour::new(
                local_public_key.to_peer_id(),
                autonat::Config::default(),
            ),
            relay: Relay::new(local_peer_id, Default::default()),
            validator,
        })
    }
}
impl NetworkBehaviourEventProcess<FloodsubEvent> for MyBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        // print::p2p_event("FloodsubEvent", format!("{:?}", event));
        match event {
            FloodsubEvent::Message(message) => {
                print::p2p_event(
                    "FloodsubEvent::Message",
                    String::from_utf8_lossy(&message.data).green().to_string(),
                );
            }
            _ => {}
        }
    }
}
impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        // print::p2p_event("MdnsEvent", format!("{:?}", event));
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}
impl NetworkBehaviourEventProcess<PingEvent> for MyBehaviour {
    fn inject_event(&mut self, _event: PingEvent) {
        // print::p2p_event("PingEvent", format!("{:?}", event));
    }
}
impl NetworkBehaviourEventProcess<IdentifyEvent> for MyBehaviour {
    fn inject_event(&mut self, _event: IdentifyEvent) {
        // print::p2p_event("IdentifyEvent", format!("{:?}", event));
    }
}
impl NetworkBehaviourEventProcess<GossipsubEvent> for MyBehaviour {
    fn inject_event(&mut self, event: GossipsubEvent) {
        // print::p2p_event("GossipsubEvent", format!("{:?}", event));
        match event {
            GossipsubEvent::Message { message, .. } => {
                match Validator::gossipsub_message_handler(self, message) {
                    Err(err) => println!("{}", err),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
impl NetworkBehaviourEventProcess<autonat::Event> for MyBehaviour {
    fn inject_event(&mut self, event: autonat::Event) {
        print::p2p_event("autonat::Event", format!("{:?}", event));
    }
}
impl NetworkBehaviourEventProcess<relay::Event> for MyBehaviour {
    fn inject_event(&mut self, event: relay::Event) {
        print::p2p_event("relay::Event", format!("{:?}", event));
    }
}
pub async fn swarm(validator: Validator) -> Result<Swarm<MyBehaviour>, Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let transport = libp2p::development_transport(local_key.clone()).await?;
    let mut behaviour = MyBehaviour::new(local_key, validator).await?;
    for ident_topic in [
        IdentTopic::new("block"),
        IdentTopic::new("stake"),
        IdentTopic::new("transaction"),
        IdentTopic::new("ip"),
        IdentTopic::new("sync"),
    ]
    .iter()
    {
        behaviour.gossipsub.subscribe(ident_topic)?;
    }
    Ok(Swarm::new(transport, behaviour, local_peer_id))
}
