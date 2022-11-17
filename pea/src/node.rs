use crate::{
    blockchain::Blockchain,
    gossipsub, heartbeat, http,
    p2p::{MyBehaviour, MyBehaviourEvent},
};
use colored::*;
use futures::{FutureExt, StreamExt};
use libp2p::{
    core::connection::ConnectedPoint,
    gossipsub::{GossipsubEvent, IdentTopic},
    mdns::MdnsEvent,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use log::{debug, error, info};
use pea_core::{types, util};
use pea_db as db;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};
use tokio::net::TcpListener;
pub struct Node {
    pub swarm: Swarm<MyBehaviour>,
    pub blockchain: Blockchain,
    pub message_data_hashes: Vec<types::Hash>,
    pub heartbeats: usize,
    pub lag: f64,
    pub tps: f64,
    pub new_multiaddrs: HashSet<Multiaddr>,
    pub peers: HashMap<Multiaddr, PeerId>,
}
impl Node {
    pub fn new(swarm: Swarm<MyBehaviour>, blockchain: Blockchain, tps: f64) -> Node {
        Node {
            swarm,
            blockchain,
            message_data_hashes: vec![],
            heartbeats: 0,
            lag: 0.0,
            tps,
            new_multiaddrs: HashSet::new(),
            peers: HashMap::new(),
        }
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
    pub async fn listen(&mut self, tcp_listener_http_api: Option<TcpListener>) -> Result<(), Box<dyn Error>> {
        if let Some(listener) = tcp_listener_http_api {
            info!("{} {} http://{}", "Enabled".green(), "HTTP API".cyan(), listener.local_addr()?.to_string().green());
            loop {
                tokio::select! {
                    Ok(stream) = http::next(&listener).fuse() => if let Err(err) = http::handler(stream, self).await {
                        error!("{}", err);
                    },
                    _ = heartbeat::next(self.tps).fuse() => heartbeat::handler(self),
                    event = self.swarm.select_next_some() => {
                        match event {
                            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                                Self::connection_established(self, peer_id, endpoint);
                            }
                            SwarmEvent::ConnectionClosed { endpoint, .. } => {
                                Self::connection_closed(self, endpoint);
                            }
                            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(MdnsEvent::Discovered(list))) => {
                                for (_, multiaddr) in list {
                                    if let Some(multiaddr) = Self::multiaddr_ip(multiaddr) {
                                        self.new_multiaddrs.insert(multiaddr);
                                    }
                                }
                            },
                            SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(GossipsubEvent::Message { message, .. })) => {
                                if self.filter(&message.data, false) {
                                    continue;
                                }
                                if let Err(err) = gossipsub::handler(self, message) {
                                    debug!("{}", err)
                                }
                            }
                            _ => {}
                        }
                    },
                }
            }
        } else {
            info!("{} {}", "HTTP API".cyan(), "Disabled".red());
            loop {
                tokio::select! {
                    _ = heartbeat::next(self.tps).fuse() => heartbeat::handler(self),
                    event = self.swarm.select_next_some() => {
                        match event {
                            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                                Self::connection_established(self, peer_id, endpoint);
                            }
                            SwarmEvent::ConnectionClosed { endpoint, .. } => {
                                Self::connection_closed(self, endpoint);
                            }
                            SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(GossipsubEvent::Message { message, .. })) => {
                                if self.filter(&message.data, false) {
                                    continue;
                                }
                                if let Err(err) = gossipsub::handler(self, message) {
                                    debug!("{}", err)
                                }
                            }
                            _ => {}
                        }
                    },
                }
            }
        }
    }
    fn connection_established(node: &mut Node, peer_id: PeerId, endpoint: ConnectedPoint) {
        let mut save = |multiaddr: Multiaddr| {
            if let Some(multiaddr) = Self::multiaddr_ip(multiaddr) {
                if let Some(peer_id) = node.peers.insert(multiaddr.clone(), peer_id) {
                    let _ = node.swarm.disconnect_peer_id(peer_id);
                }
                let timestamp = util::timestamp();
                let bytes = timestamp.to_le_bytes();
                let _ = db::peer::put(&multiaddr.to_string(), &bytes, &node.blockchain.db);
                if node.swarm.behaviour().gossipsub.all_peers().count() == 0 {
                    return;
                }
                let data = bincode::serialize(&multiaddr).unwrap();
                if let Err(err) = node.swarm.behaviour_mut().gossipsub.publish(IdentTopic::new("multiaddr"), data) {
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
    fn connection_closed(node: &mut Node, endpoint: ConnectedPoint) {
        let mut save = |multiaddr: Multiaddr| {
            if let Some(multiaddr) = Self::multiaddr_ip(multiaddr) {
                node.peers.remove(&multiaddr);
                let _ = node.swarm.dial(multiaddr);
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
}
