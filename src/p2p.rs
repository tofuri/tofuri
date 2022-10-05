use crate::{
    blockchain::Blockchain, constants::PROTOCOL_VERSION, gossipsub, heartbeat, http, print, types,
    util,
};
use colored::*;
use libp2p::{
    autonat,
    floodsub::{Floodsub, FloodsubEvent},
    futures::{FutureExt, StreamExt},
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubEvent, IdentTopic, MessageAuthenticity,
    },
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    identity,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    ping::{self, Ping, PingEvent},
    relay::v2::relay::{self, Relay},
    swarm::NetworkBehaviourEventProcess,
    NetworkBehaviour, PeerId, Swarm,
};
use log::error;
use std::{error::Error, time::Duration};
use tokio::net::TcpListener;
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
    pub blockchain: Blockchain,
    #[behaviour(ignore)]
    pub message_data_hashes: Vec<types::Hash>,
}
impl MyBehaviour {
    async fn new(
        local_key: identity::Keypair,
        blockchain: Blockchain,
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
            blockchain,
            message_data_hashes: vec![],
        })
    }
    pub fn filter(&mut self, data: &[u8], save: bool) -> bool {
        let hash = util::hash(data);
        if self.message_data_hashes.contains(&hash) {
            return true;
        }
        if save {
            self.message_data_hashes.push(hash);
        }
        false
    }
}
impl NetworkBehaviourEventProcess<FloodsubEvent> for MyBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        // print::p2p_event("FloodsubEvent", format!("{:?}", event));
        if let FloodsubEvent::Message(message) = event {
            print::p2p_event(
                "FloodsubEvent::Message",
                String::from_utf8_lossy(&message.data).green().to_string(),
            );
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
        if let GossipsubEvent::Message { message, .. } = event {
            if self.filter(&message.data, false) {
                return;
            }
            if let Err(err) = gossipsub::handler(self, message) {
                error!("{}", err)
            }
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
pub async fn swarm(blockchain: Blockchain) -> Result<Swarm<MyBehaviour>, Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let transport = libp2p::development_transport(local_key.clone()).await?;
    let mut behaviour = MyBehaviour::new(local_key, blockchain).await?;
    for ident_topic in [
        IdentTopic::new("block"),
        IdentTopic::new("stake"),
        IdentTopic::new("transaction"),
    ]
    .iter()
    {
        behaviour.gossipsub.subscribe(ident_topic)?;
    }
    Ok(Swarm::new(transport, behaviour, local_peer_id))
}
pub async fn listen(
    swarm: &mut Swarm<MyBehaviour>,
    listener: TcpListener,
) -> Result<(), Box<dyn Error>> {
    loop {
        tokio::select! {
            _ = heartbeat::next().fuse() => if let Err(err) = heartbeat::handler(swarm) {
                error!("{}", err);
            },
            Ok(stream) = http::next(&listener).fuse() => if let Err(err) = http::handler(stream, swarm).await {
                error!("{}", err);
            },
            event = swarm.select_next_some() => print::p2p_event("SwarmEvent", format!("{:?}", event)),
        }
    }
}
