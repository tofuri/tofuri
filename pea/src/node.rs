use crate::http;
use colored::*;
use libp2p::core::connection::ConnectedPoint;
use libp2p::core::either::EitherError;
use libp2p::futures::StreamExt;
use libp2p::gossipsub::error::GossipsubHandlerError;
use libp2p::gossipsub::GossipsubEvent;
use libp2p::gossipsub::GossipsubMessage;
use libp2p::mdns;
use libp2p::multiaddr::Protocol;
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
use pea_blockchain::blockchain::Blockchain;
use pea_core::*;
use pea_db as db;
use pea_key::Key;
use pea_p2p::behaviour::OutEvent;
use pea_p2p::behaviour::SyncRequest;
use pea_p2p::behaviour::SyncResponse;
use pea_p2p::multiaddr;
use pea_p2p::ratelimit::Endpoint;
use pea_p2p::P2p;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use pea_util;
use pea_wallet::wallet;
use rand::prelude::*;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use serde::Deserialize;
use serde::Serialize;
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
    pub db: DBWithThreadMode<SingleThreaded>,
    pub key: Key,
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
        if let Some(multiaddr) = multiaddr::ip_port(&options.peer.parse::<Multiaddr>().unwrap()) {
            known.insert(multiaddr);
        }
        let peers = db::peer::get_all(&db);
        for peer in peers {
            if let Some(multiaddr) = multiaddr::ip_port(&peer.parse::<Multiaddr>().unwrap()) {
                known.insert(multiaddr);
            }
        }
        let p2p = P2p::new(options.max_established, options.timeout, known, options.ban_offline).await.unwrap();
        let blockchain = Blockchain::new(options.trust, options.time_delta);
        Node {
            key,
            p2p,
            blockchain,
            db,
            heartbeats: 0,
            lag: 0.0,
            options,
        }
    }
    pub async fn run(&mut self) {
        self.blockchain.load(&self.db);
        info!(
            "Blockchain height is {}",
            if let Some(main) = self.blockchain.tree.main() {
                main.1.to_string().yellow()
            } else {
                "0".red()
            }
        );
        info!("Latest block seen {}", self.blockchain.last_seen().yellow());
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
                instant = interval.tick() => self.heartbeat(instant),
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
    pub fn uptime(&self) -> String {
        let seconds = (self.heartbeats as f64 / self.options.tps) as u32;
        pea_util::duration_to_string(seconds, "0")
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
                Self::swarm_event_connection_established(self, peer_id, endpoint, num_established);
            }
            SwarmEvent::ConnectionClosed { endpoint, num_established, .. } => {
                Self::swarm_event_connection_closed(self, endpoint, num_established);
            }
            SwarmEvent::Behaviour(OutEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (_, multiaddr) in list {
                    if let Some(multiaddr) = multiaddr::ip_port(&multiaddr) {
                        self.p2p.unknown.insert(multiaddr);
                    }
                }
            }
            SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                message, propagation_source, ..
            })) => {
                if let Err(err) = self.swarm_event_gossipsub_message(message, propagation_source) {
                    error!("GossipsubEvent::Message {}", err)
                }
            }
            SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message { message, peer })) => match message {
                RequestResponseMessage::Request { request, channel, .. } => {
                    if let Err(err) = self.swarm_event_request(peer, request, channel) {
                        error!("RequestResponseMessage::Request {}", err)
                    }
                }
                RequestResponseMessage::Response { response, .. } => {
                    if let Err(err) = self.swarm_event_response(peer, response) {
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
    fn swarm_event_connection_established(&mut self, peer_id: PeerId, endpoint: ConnectedPoint, num_established: NonZeroU32) {
        let mut save = |multiaddr: Multiaddr| {
            info!(
                "Connection {} {} {}",
                "established".green(),
                multiaddr.to_string().magenta(),
                num_established.to_string().yellow()
            );
            let addr = multiaddr::ip_addr(&multiaddr).expect("multiaddr to include ip");
            if self.p2p.ratelimit.is_ratelimited(&self.p2p.ratelimit.get(&addr).1) {
                warn!("Ratelimited {}", multiaddr.to_string().magenta());
                let _ = self.p2p.swarm.disconnect_peer_id(peer_id);
            }
            self.p2p.known.insert(multiaddr.clone());
            let _ = db::peer::put(&multiaddr.to_string(), &self.db);
            if let Some(previous_peer_id) = self
                .p2p
                .connections
                .insert(multiaddr::ip(&multiaddr).expect("multiaddr to include ip"), peer_id)
            {
                if previous_peer_id != peer_id {
                    let _ = self.p2p.swarm.disconnect_peer_id(previous_peer_id);
                }
            }
        };
        if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
            if let Some(multiaddr) = multiaddr::ip_port(&address) {
                save(multiaddr);
            }
        }
        if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
            if let Some(multiaddr) = multiaddr::ip(&send_back_addr) {
                save(multiaddr);
            }
        }
    }
    fn swarm_event_connection_closed(&mut self, endpoint: ConnectedPoint, num_established: u32) {
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
            if let Some(multiaddr) = multiaddr::ip_port(&address) {
                save(multiaddr);
            }
        }
        if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
            if let Some(multiaddr) = multiaddr::ip(&send_back_addr) {
                save(multiaddr);
            }
        }
    }
    fn swarm_event_gossipsub_message(&mut self, message: GossipsubMessage, propagation_source: PeerId) -> Result<(), Box<dyn std::error::Error>> {
        match message.topic.as_str() {
            "block" => {
                self.p2p.ratelimit(propagation_source, Endpoint::Block)?;
                if self.p2p.filter(&message.data) {
                    return Err("filter block".into());
                }
                let block_b: BlockB = bincode::deserialize(&message.data)?;
                self.blockchain.append_block(&self.db, block_b, pea_util::timestamp())?;
            }
            "transaction" => {
                self.p2p.ratelimit(propagation_source, Endpoint::Transaction)?;
                if self.p2p.filter(&message.data) {
                    return Err("filter transaction".into());
                }
                let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
                self.blockchain.pending_transactions_push(&self.db, transaction_b, pea_util::timestamp())?;
            }
            "stake" => {
                self.p2p.ratelimit(propagation_source, Endpoint::Stake)?;
                if self.p2p.filter(&message.data) {
                    return Err("filter stake".into());
                }
                let stake_b: StakeB = bincode::deserialize(&message.data)?;
                self.blockchain.pending_stakes_push(&self.db, stake_b, pea_util::timestamp())?;
            }
            "multiaddr" => {
                self.p2p.ratelimit(propagation_source, Endpoint::Multiaddr)?;
                if self.p2p.filter(&message.data) {
                    return Err("filter multiaddr".into());
                }
                for multiaddr in bincode::deserialize::<Vec<Multiaddr>>(&message.data)? {
                    if let Some(multiaddr) = multiaddr::ip_port(&multiaddr) {
                        self.p2p.unknown.insert(multiaddr);
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }
    fn swarm_event_request(&mut self, peer_id: PeerId, request: SyncRequest, channel: ResponseChannel<SyncResponse>) -> Result<(), Box<dyn Error>> {
        self.p2p.ratelimit(peer_id, Endpoint::SyncRequest)?;
        let height: usize = bincode::deserialize(&request.0)?;
        let mut vec = vec![];
        for i in 0..SYNC_BLOCKS_PER_TICK {
            match self.blockchain.sync_block(&self.db, height + i) {
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
    fn swarm_event_response(&mut self, peer_id: PeerId, response: SyncResponse) -> Result<(), Box<dyn Error>> {
        self.p2p.ratelimit(peer_id, Endpoint::SyncResponse)?;
        let timestamp = pea_util::timestamp();
        for block_b in bincode::deserialize::<Vec<BlockB>>(&response.0)? {
            if let Err(err) = self.blockchain.append_block(&self.db, block_b, timestamp) {
                debug!("response_handler {}", err);
            }
        }
        Ok(())
    }
    fn heartbeat_delay(&self, seconds: usize) -> bool {
        (self.heartbeats as f64 % (self.options.tps * seconds as f64)) as usize == 0
    }
    fn heartbeat(&mut self, instant: tokio::time::Instant) {
        let timestamp = pea_util::timestamp();
        if self.heartbeat_delay(60) {
            self.heartbeat_dial_known();
        }
        if self.heartbeat_delay(10) {
            self.heartbeat_share();
        }
        if self.heartbeat_delay(5) {
            self.heartbeat_dial_unknown();
        }
        if self.heartbeat_delay(1) {
            self.blockchain.sync.handler();
            self.p2p.ratelimit.reset();
            self.p2p.filter.clear();
        }
        self.heartbeat_sync_request();
        self.heartbeat_offline_staker(timestamp);
        self.heartbeat_grow(timestamp);
        self.heartbeats += 1;
        self.heartbeat_lag(instant.elapsed());
    }
    fn heartbeat_offline_staker(&mut self, timestamp: u32) {
        if self.p2p.ban_offline == 0 {
            return;
        }
        if !self.blockchain.sync.completed {
            return;
        }
        if self.p2p.connections.len() < self.p2p.ban_offline {
            return;
        }
        let dynamic = &self.blockchain.states.dynamic;
        for staker in dynamic.stakers_offline(timestamp, dynamic.latest_block.timestamp) {
            if let Some(hash) = self.blockchain.offline.insert(staker, dynamic.latest_block.hash) {
                if hash == dynamic.latest_block.hash {
                    return;
                }
            }
            warn!("Banned offline staker {}", address::encode(&staker).green());
        }
    }
    fn heartbeat_dial_known(&mut self) {
        let vec = self.p2p.known.clone().into_iter().collect();
        self.heartbeat_dial(vec, true);
    }
    fn heartbeat_dial_unknown(&mut self) {
        let vec = self.p2p.unknown.drain().collect();
        self.heartbeat_dial(vec, false);
    }
    fn heartbeat_dial(&mut self, vec: Vec<Multiaddr>, known: bool) {
        for mut multiaddr in vec {
            if self.p2p.connections.contains_key(&multiaddr::ip(&multiaddr).expect("multiaddr to include ip")) {
                continue;
            }
            let addr = multiaddr::ip_addr(&multiaddr).expect("multiaddr to include ip");
            if self.p2p.ratelimit.is_ratelimited(&self.p2p.ratelimit.get(&addr).1) {
                continue;
            }
            debug!(
                "Dialing {} peer {}",
                if known { "known".green() } else { "unknown".red() },
                multiaddr.to_string().magenta()
            );
            if !multiaddr::has_port(&multiaddr) {
                multiaddr.push(Protocol::Tcp(9333));
            }
            let _ = self.p2p.swarm.dial(multiaddr);
        }
    }
    fn heartbeat_share(&mut self) {
        if !self.p2p.gossipsub_has_mesh_peers("multiaddr") {
            return;
        }
        let vec: Vec<&Multiaddr> = self.p2p.connections.keys().collect();
        if let Err(err) = self.p2p.gossipsub_publish("multiaddr", bincode::serialize(&vec).unwrap()) {
            error!("{}", err);
        }
    }
    fn heartbeat_grow(&mut self, timestamp: u32) {
        if !self.blockchain.sync.downloading() && !self.options.mint && self.blockchain.states.dynamic.next_staker(timestamp).is_none() {
            if self.heartbeat_delay(60) {
                info!(
                    "Waiting for synchronization to start... Currently connected to {} peers.",
                    self.p2p.connections.len().to_string().yellow()
                );
            }
            self.blockchain.sync.completed = false;
        }
        if !self.blockchain.sync.completed {
            return;
        }
        if let Some(block_a) = self.blockchain.forge_block(&self.db, &self.key, timestamp) {
            if !self.p2p.gossipsub_has_mesh_peers("block") {
                return;
            }
            if let Err(err) = self.p2p.gossipsub_publish("block", bincode::serialize(&block_a.b()).unwrap()) {
                error!("{}", err);
            }
        }
    }
    fn heartbeat_sync_request(&mut self) {
        if let Some(peer_id) = self.p2p.swarm.connected_peers().choose(&mut thread_rng()).cloned() {
            self.p2p
                .swarm
                .behaviour_mut()
                .request_response
                .send_request(&peer_id, SyncRequest(bincode::serialize(&(self.blockchain.height())).unwrap()));
        }
    }
    fn heartbeat_lag(&mut self, duration: Duration) {
        self.lag = duration.as_micros() as f64 / 1_000_f64;
        debug!("{} {} {}", "Heartbeat".cyan(), self.heartbeats, format!("{duration:?}").yellow());
    }
}
