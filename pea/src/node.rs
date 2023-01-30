use crate::blockchain::Blockchain;
use crate::heartbeat;
use crate::http;
use colored::*;
use libp2p::core::connection::ConnectedPoint;
use libp2p::core::either::EitherError;
use libp2p::futures::StreamExt;
use libp2p::gossipsub::error::GossipsubHandlerError;
use libp2p::gossipsub::GossipsubEvent;
use libp2p::gossipsub::GossipsubMessage;
use libp2p::gossipsub::IdentTopic;
use libp2p::gossipsub::TopicHash;
use libp2p::mdns;
use libp2p::request_response::RequestResponseEvent;
use libp2p::request_response::RequestResponseMessage;
use libp2p::request_response::ResponseChannel;
use libp2p::swarm::ConnectionHandlerUpgrErr;
use libp2p::swarm::SwarmEvent;
use libp2p::Multiaddr;
use libp2p::PeerId;
use log::debug;
use log::error;
use log::info;
use log::warn;
use pea_address::address;
use pea_block::BlockB;
use pea_core::*;
use pea_db as db;
use pea_key::Key;
use pea_p2p::behaviour::OutEvent;
use pea_p2p::behaviour::SyncRequest;
use pea_p2p::behaviour::SyncResponse;
use pea_p2p::ratelimit::Endpoint;
use pea_p2p::P2p;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use pea_util;
use pea_wallet::wallet;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashSet;
use std::error::Error;
use std::io;
use std::num::NonZeroU32;
use std::time::Duration;
use tempdir::TempDir;
use tokio::net::TcpListener;
use void::Void;
type HandlerErr = EitherError<
    EitherError<EitherError<EitherError<Void, io::Error>, GossipsubHandlerError>, ConnectionHandlerUpgrErr<io::Error>>,
    ConnectionHandlerUpgrErr<io::Error>,
>;
#[derive(Serialize, Deserialize, Debug)]
pub struct Options<'a> {
    pub tempdb: bool,
    pub tempkey: bool,
    pub mint: bool,
    pub time_api: bool,
    pub trust: usize,
    pub ban_offline: usize,
    pub time_delta: u32,
    pub max_established: Option<u32>,
    pub tps: f64,
    pub wallet: &'a str,
    pub passphrase: &'a str,
    pub peer: &'a str,
    pub bind_api: &'a str,
    pub host: &'a str,
    pub dev: bool,
    pub timeout: u64,
}
pub struct Node<'a> {
    pub options: Options<'a>,
    pub p2p: P2p,
    pub blockchain: Blockchain,
    pub heartbeats: usize,
    pub lag: f64,
}
impl Node<'_> {
    pub async fn new(options: Options<'_>) -> Node<'_> {
        let key = match options.tempkey {
            true => Key::generate(),
            false => wallet::load(options.wallet, options.passphrase).unwrap().3,
        };
        info!("Address {}", address::encode(&key.address_bytes()).green());
        let tempdir = TempDir::new("peacash-db").unwrap();
        let path: &str = match options.tempdb {
            true => tempdir.path().to_str().unwrap(),
            false => "./peacash-db",
        };
        let db = db::open(path);
        let mut known = HashSet::new();
        if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip_port(&options.peer.parse::<Multiaddr>().unwrap()) {
            known.insert(multiaddr);
        }
        let peers = db::peer::get_all(&db);
        for peer in peers {
            if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip_port(&peer.parse::<Multiaddr>().unwrap()) {
                known.insert(multiaddr);
            }
        }
        let p2p = P2p::new(options.max_established, options.timeout, known, options.ban_offline).await.unwrap();
        let blockchain = Blockchain::new(db, key, options.trust, options.time_delta);
        Node {
            p2p,
            blockchain,
            heartbeats: 0,
            lag: 0.0,
            options,
        }
    }
    pub fn filter(&mut self, data: &[u8], save: bool) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize().into();
        if self.p2p.message_data_hashes.contains(&hash) {
            return true;
        }
        if save {
            self.p2p.message_data_hashes.push(hash);
        }
        false
    }
    fn swarm_event(&mut self, event: SwarmEvent<OutEvent, HandlerErr>) {
        debug!("{:?}", event);
        match event {
            SwarmEvent::Dialing(_) => {}
            SwarmEvent::IncomingConnectionError { .. } => {}
            SwarmEvent::IncomingConnection { .. } => {}
            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                ..
            } => {
                Self::connection_established(self, peer_id, endpoint, num_established);
            }
            SwarmEvent::ConnectionClosed { endpoint, num_established, .. } => {
                Self::connection_closed(self, endpoint, num_established);
            }
            SwarmEvent::Behaviour(OutEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (_, multiaddr) in list {
                    if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip_port(&multiaddr) {
                        self.p2p.unknown.insert(multiaddr);
                    }
                }
            }
            SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                message, propagation_source, ..
            })) => {
                if self.filter(&message.data, false) {
                    return;
                }
                if let Err(err) = self.gossipsub_handler(message, propagation_source) {
                    error!("GossipsubEvent::Message {}", err)
                }
            }
            SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message { message, peer })) => match message {
                RequestResponseMessage::Request { request, channel, .. } => {
                    if let Err(err) = self.request_handler(peer, request, channel) {
                        error!("RequestResponseMessage::Request {}", err)
                    }
                }
                RequestResponseMessage::Response { response, .. } => {
                    if let Err(err) = self.response_handler(peer, response) {
                        error!("RequestResponseMessage::Response {}", err)
                    }
                }
            },
            SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::InboundFailure { .. })) => {}
            SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::OutboundFailure { .. })) => {}
            SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::ResponseSent { .. })) => {}
            _ => {}
        }
    }
    pub async fn start(&mut self) {
        self.blockchain.load();
        info!(
            "Blockchain height is {}",
            if let Some(main) = self.blockchain.tree.main() {
                main.1.to_string().yellow()
            } else {
                "0".red()
            }
        );
        info!("Latest block seen {}", self.last_seen().yellow());
        let multiaddr: Multiaddr = self.options.host.parse().unwrap();
        self.p2p.swarm.listen_on(multiaddr.clone()).unwrap();
        info!("Swarm is listening on {}", multiaddr.to_string().magenta());
        let listener = TcpListener::bind(self.options.bind_api).await.unwrap();
        info!(
            "API is listening on {}{}",
            "http://".cyan(),
            listener.local_addr().unwrap().to_string().magenta()
        );
        let mut interval = tokio::time::interval(Duration::from_micros(pea_util::micros_per_tick(self.options.tps)));
        loop {
            tokio::select! {
                biased;
                instant = interval.tick() => heartbeat::handler(self, instant),
                event = self.p2p.swarm.select_next_some() => self.swarm_event(event),
                res = listener.accept() => match res {
                    Ok((stream, socket_addr)) => {
                        match http::handler(stream, self).await {
                            Ok((bytes, first)) => info!("{} {} {} {}", "API".cyan(), socket_addr.to_string().magenta(), bytes.to_string().yellow(), first),
                            Err(err) => error!("{} {} {}", "API".cyan(), socket_addr.to_string().magenta(), err)
                        }
                    }
                    Err(err) => error!("{} {}", "API".cyan(), err)
                }
            }
        }
    }
    fn connection_established(&mut self, peer_id: PeerId, endpoint: ConnectedPoint, num_established: NonZeroU32) {
        let mut save = |multiaddr: Multiaddr| {
            info!(
                "Connection {} {} {}",
                "established".green(),
                multiaddr.to_string().magenta(),
                num_established.to_string().yellow()
            );
            let addr = pea_p2p::multiaddr::multiaddr_addr(&multiaddr).expect("multiaddr to include ip");
            if self.p2p.ratelimit.is_ratelimited(&self.p2p.ratelimit.get(&addr).1) {
                warn!("Ratelimited {}", multiaddr.to_string().magenta());
                let _ = self.p2p.swarm.disconnect_peer_id(peer_id);
            }
            self.p2p.known.insert(multiaddr.clone());
            let _ = db::peer::put(&multiaddr.to_string(), &self.blockchain.db);
            if let Some(previous_peer_id) = self
                .p2p
                .connections
                .insert(pea_p2p::multiaddr::multiaddr_filter_ip(&multiaddr).expect("multiaddr to include ip"), peer_id)
            {
                if previous_peer_id != peer_id {
                    let _ = self.p2p.swarm.disconnect_peer_id(previous_peer_id);
                }
            }
        };
        if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
            if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip_port(&address) {
                save(multiaddr);
            }
        }
        if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
            if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip(&send_back_addr) {
                save(multiaddr);
            }
        }
    }
    fn connection_closed(&mut self, endpoint: ConnectedPoint, num_established: u32) {
        let mut save = |multiaddr: Multiaddr| {
            info!(
                "Connection {} {} {}",
                "closed".red(),
                multiaddr.to_string().magenta(),
                num_established.to_string().yellow()
            );
            self.p2p.connections.remove(&multiaddr);
            let _ = self.p2p.swarm.dial(multiaddr);
        };
        if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
            if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip_port(&address) {
                save(multiaddr);
            }
        }
        if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
            if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip(&send_back_addr) {
                save(multiaddr);
            }
        }
    }
    pub fn gossipsub_has_mesh_peers(&mut self, topic: &str) -> bool {
        self.p2p.swarm.behaviour().gossipsub.mesh_peers(&TopicHash::from_raw(topic)).count() != 0
    }
    pub fn gossipsub_publish(&mut self, topic: &str, data: Vec<u8>) {
        self.filter(&data, true);
        if let Err(err) = self.p2p.swarm.behaviour_mut().gossipsub.publish(IdentTopic::new(topic), data) {
            error!("{}", err);
        }
    }
    pub fn last_seen(&self) -> String {
        if self.blockchain.states.dynamic.latest_block.timestamp == 0 {
            return "never".to_string();
        }
        let timestamp = self.blockchain.states.dynamic.latest_block.timestamp;
        let diff = pea_util::timestamp().saturating_sub(timestamp);
        let now = "just now";
        let mut string = pea_util::duration_to_string(diff, now);
        if string != now {
            string.push_str(" ago");
        }
        string
    }
    pub fn sync_status(&self) -> String {
        let completed = "completed";
        if self.blockchain.sync.completed {
            return completed.to_string();
        }
        if !self.blockchain.sync.downloading() {
            return "waiting to start".to_string();
        }
        let timestamp = self.blockchain.states.dynamic.latest_block.timestamp;
        let mut diff = pea_util::timestamp().saturating_sub(timestamp) as f32;
        diff /= BLOCK_TIME_MIN as f32;
        diff /= self.blockchain.sync.bps;
        let mut string = pea_util::duration_to_string(diff as u32, completed);
        if string != completed {
            string.push_str(" remaining");
        }
        string
    }
    pub fn uptime(&self) -> String {
        let seconds = (self.heartbeats as f64 / self.options.tps) as u32;
        pea_util::duration_to_string(seconds, "0")
    }
    pub fn gossipsub_handler(&mut self, message: GossipsubMessage, propagation_source: PeerId) -> Result<(), Box<dyn std::error::Error>> {
        match message.topic.as_str() {
            "block" => {
                self.p2p.ratelimit(propagation_source, Endpoint::Block)?;
                let block_b: BlockB = bincode::deserialize(&message.data)?;
                self.blockchain.append_block(block_b, pea_util::timestamp())?;
            }
            "transaction" => {
                self.p2p.ratelimit(propagation_source, Endpoint::Transaction)?;
                let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
                self.blockchain.pending_transactions_push(transaction_b, pea_util::timestamp())?;
            }
            "stake" => {
                self.p2p.ratelimit(propagation_source, Endpoint::Stake)?;
                let stake_b: StakeB = bincode::deserialize(&message.data)?;
                self.blockchain.pending_stakes_push(stake_b, pea_util::timestamp())?;
            }
            "multiaddr" => {
                self.p2p.ratelimit(propagation_source, Endpoint::Multiaddr)?;
                for multiaddr in bincode::deserialize::<Vec<Multiaddr>>(&message.data)? {
                    if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip_port(&multiaddr) {
                        self.p2p.unknown.insert(multiaddr);
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }
    pub fn request_handler(&mut self, peer_id: PeerId, request: SyncRequest, channel: ResponseChannel<SyncResponse>) -> Result<(), Box<dyn Error>> {
        self.p2p.ratelimit(peer_id, Endpoint::SyncRequest)?;
        let height: usize = bincode::deserialize(&request.0)?;
        let mut vec = vec![];
        for i in 0..SYNC_BLOCKS_PER_TICK {
            match self.blockchain.sync_block(height + i) {
                Some(block_b) => vec.push(block_b),
                None => break,
            }
        }
        if self
            .p2p
            .swarm
            .behaviour_mut()
            .request_response
            .send_response(channel, SyncResponse(bincode::serialize(&vec).unwrap()))
            .is_err()
        {
            return Err("p2p request handler connection closed".into());
        };
        Ok(())
    }
    pub fn response_handler(&mut self, peer_id: PeerId, response: SyncResponse) -> Result<(), Box<dyn Error>> {
        self.p2p.ratelimit(peer_id, Endpoint::SyncResponse)?;
        let timestamp = pea_util::timestamp();
        for block_b in bincode::deserialize::<Vec<BlockB>>(&response.0)? {
            if let Err(err) = self.blockchain.append_block(block_b, timestamp) {
                debug!("response_handler {}", err);
            }
        }
        Ok(())
    }
}
