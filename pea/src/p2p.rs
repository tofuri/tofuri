use crate::{blockchain::Blockchain, gossipsub, heartbeat, http};
use colored::*;
use futures::{FutureExt, StreamExt};
use libp2p::{
    autonat,
    core::connection::ConnectedPoint,
    gossipsub::{Gossipsub, GossipsubConfigBuilder, GossipsubEvent, IdentTopic, MessageAuthenticity},
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    identity,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    ping::{self, Ping, PingEvent},
    swarm::{NetworkBehaviourEventProcess, SwarmEvent},
    Multiaddr, NetworkBehaviour, PeerId, Swarm,
};
use log::{debug, error, info};
use pea_core::{constants::PROTOCOL_VERSION, types, util};
use pea_db as db;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    time::Duration,
};
use tokio::net::TcpListener;
#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct MyBehaviour {
    pub mdns: Mdns,
    pub ping: Ping,
    pub identify: Identify,
    pub gossipsub: Gossipsub,
    pub autonat: autonat::Behaviour,
    #[behaviour(ignore)]
    pub blockchain: Blockchain,
    #[behaviour(ignore)]
    pub message_data_hashes: Vec<types::Hash>,
    #[behaviour(ignore)]
    pub heartbeats: usize,
    #[behaviour(ignore)]
    pub lag: f64,
    #[behaviour(ignore)]
    pub tps: f64,
    #[behaviour(ignore)]
    pub new_multiaddrs: HashSet<Multiaddr>,
    #[behaviour(ignore)]
    pub peers: HashMap<Multiaddr, PeerId>,
}
impl MyBehaviour {
    async fn new(local_key: identity::Keypair, blockchain: Blockchain, tps: f64) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            mdns: Mdns::new(MdnsConfig::default()).await?,
            ping: ping::Behaviour::new(ping::Config::new().with_keep_alive(true)),
            identify: Identify::new(IdentifyConfig::new(PROTOCOL_VERSION.to_string(), local_key.public())),
            gossipsub: Gossipsub::new(
                MessageAuthenticity::Signed(local_key.clone()),
                GossipsubConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                    .build()?,
            )?,
            autonat: autonat::Behaviour::new(local_key.public().to_peer_id(), autonat::Config::default()),
            blockchain,
            message_data_hashes: vec![],
            heartbeats: 0,
            lag: 0.0,
            tps,
            new_multiaddrs: HashSet::new(),
            peers: HashMap::new(),
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
impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        debug!("{:?}", event);
    }
}
impl NetworkBehaviourEventProcess<PingEvent> for MyBehaviour {
    fn inject_event(&mut self, event: PingEvent) {
        debug!("{:?}", event);
    }
}
impl NetworkBehaviourEventProcess<IdentifyEvent> for MyBehaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        debug!("{:?}", event);
    }
}
impl NetworkBehaviourEventProcess<GossipsubEvent> for MyBehaviour {
    fn inject_event(&mut self, event: GossipsubEvent) {
        debug!("{:?}", event);
        if let GossipsubEvent::Message { message, .. } = event {
            if self.filter(&message.data, false) {
                return;
            }
            if let Err(err) = gossipsub::handler(self, message) {
                debug!("{}", err)
            }
        }
    }
}
impl NetworkBehaviourEventProcess<autonat::Event> for MyBehaviour {
    fn inject_event(&mut self, event: autonat::Event) {
        debug!("{:?}", event);
    }
}
pub async fn swarm(blockchain: Blockchain, tps: f64) -> Result<Swarm<MyBehaviour>, Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let transport = libp2p::development_transport(local_key.clone()).await?;
    let mut behaviour = MyBehaviour::new(local_key, blockchain, tps).await?;
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
    Ok(Swarm::new(transport, behaviour, local_peer_id))
}
pub async fn listen(swarm: &mut Swarm<MyBehaviour>, tcp_listener_http_api: Option<TcpListener>) -> Result<(), Box<dyn Error>> {
    if let Some(listener) = tcp_listener_http_api {
        info!("{} {} http://{}", "Enabled".green(), "HTTP API".cyan(), listener.local_addr()?.to_string().green());
        loop {
            tokio::select! {
                Ok(stream) = http::next(&listener).fuse() => if let Err(err) = http::handler(stream, swarm).await {
                    error!("{}", err);
                },
                _ = heartbeat::next(swarm.behaviour().tps).fuse() => heartbeat::handler(swarm),
                event = swarm.select_next_some() => {
                    debug!("{:?}", event);
                    if let SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } = event {
                        connection_established(swarm, peer_id, endpoint);
                    } else if let SwarmEvent::ConnectionClosed { endpoint, .. } = event {
                        connection_closed(swarm, endpoint);
                    }
                },
            }
        }
    } else {
        info!("{} {}", "HTTP API".cyan(), "Disabled".red());
        loop {
            tokio::select! {
                _ = heartbeat::next(swarm.behaviour().tps).fuse() => heartbeat::handler(swarm),
                event = swarm.select_next_some() => {
                    debug!("{:?}", event);
                    if let SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } = event {
                        connection_established(swarm, peer_id, endpoint);
                    } else if let SwarmEvent::ConnectionClosed { endpoint, .. } = event {
                        connection_closed(swarm, endpoint);
                    }
                },
            }
        }
    }
}
fn connection_established(swarm: &mut Swarm<MyBehaviour>, peer_id: PeerId, endpoint: ConnectedPoint) {
    let mut save = |multiaddr: Multiaddr| {
        if let Some(multiaddr) = multiaddr_ip(multiaddr) {
            if let Some(peer_id) = swarm.behaviour_mut().peers.insert(multiaddr.clone(), peer_id) {
                let _ = swarm.disconnect_peer_id(peer_id);
            }
            let timestamp = util::timestamp();
            let bytes = timestamp.to_le_bytes();
            let _ = db::peer::put(&multiaddr.to_string(), &bytes, &swarm.behaviour().blockchain.db);
            if swarm.behaviour().gossipsub.all_peers().count() == 0 {
                return;
            }
            let data = bincode::serialize(&multiaddr).unwrap();
            if let Err(err) = swarm.behaviour_mut().gossipsub.publish(IdentTopic::new("multiaddr"), data) {
                error!("{}", err);
            }
        }
    };
    if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
        save(address);
    }
    if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
        save(send_back_addr);
    }
}
fn connection_closed(swarm: &mut Swarm<MyBehaviour>, endpoint: ConnectedPoint) {
    let mut save = |multiaddr: Multiaddr| {
        if let Some(multiaddr) = multiaddr_ip(multiaddr) {
            swarm.behaviour_mut().peers.remove(&multiaddr);
            let _ = swarm.dial(multiaddr);
        }
    };
    if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
        save(address);
    }
    if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
        save(send_back_addr);
    }
}
pub fn multiaddr_ip(multiaddr: Multiaddr) -> Option<Multiaddr> {
    match multiaddr.iter().next() {
        Some(ip) => Some(ip.to_string().parse().unwrap()),
        None => None,
    }
}
