use crate::P2P_RATELIMIT_GOSSIPSUB_MESSAGE_BLOCK;
use crate::P2P_RATELIMIT_GOSSIPSUB_MESSAGE_PEERS;
use crate::P2P_RATELIMIT_GOSSIPSUB_MESSAGE_STAKE;
use crate::P2P_RATELIMIT_GOSSIPSUB_MESSAGE_TRANSACTION;
use crate::P2P_RATELIMIT_REQUEST;
use crate::P2P_RATELIMIT_REQUEST_TIMEOUT;
use crate::P2P_RATELIMIT_RESPONSE;
use crate::P2P_RATELIMIT_RESPONSE_TIMEOUT;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::net::IpAddr;
pub enum Endpoint {
    Request,
    Response,
    GossipsubMessageBlock,
    GossipsubMessageTransaction,
    GossipsubMessageStake,
    GossipsubMessagePeers,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ratelimit {
    pub counter: Counter,
    pub timeout: Timeout,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Counter {
    pub request: HashMap<IpAddr, usize>,
    pub response: HashMap<IpAddr, usize>,
    pub gossipsub_message_block: HashMap<IpAddr, usize>,
    pub gossipsub_message_transaction: HashMap<IpAddr, usize>,
    pub gossipsub_message_stake: HashMap<IpAddr, usize>,
    pub gossipsub_message_peers: HashMap<IpAddr, usize>,
}
impl Counter {
    pub fn add(&mut self, ip_addr: IpAddr, endpoint: &Endpoint) -> bool {
        let map = match endpoint {
            Endpoint::Request => &mut self.request,
            Endpoint::Response => &mut self.response,
            Endpoint::GossipsubMessageBlock => &mut self.gossipsub_message_block,
            Endpoint::GossipsubMessageTransaction => &mut self.gossipsub_message_transaction,
            Endpoint::GossipsubMessageStake => &mut self.gossipsub_message_stake,
            Endpoint::GossipsubMessagePeers => &mut self.gossipsub_message_peers,
        };
        let limit = match endpoint {
            Endpoint::Request => P2P_RATELIMIT_REQUEST,
            Endpoint::Response => P2P_RATELIMIT_RESPONSE,
            Endpoint::GossipsubMessageBlock => P2P_RATELIMIT_GOSSIPSUB_MESSAGE_BLOCK,
            Endpoint::GossipsubMessageTransaction => P2P_RATELIMIT_GOSSIPSUB_MESSAGE_TRANSACTION,
            Endpoint::GossipsubMessageStake => P2P_RATELIMIT_GOSSIPSUB_MESSAGE_STAKE,
            Endpoint::GossipsubMessagePeers => P2P_RATELIMIT_GOSSIPSUB_MESSAGE_PEERS,
        };
        let mut i = *map.get(&ip_addr).unwrap_or(&0);
        i += 1;
        map.insert(ip_addr, i);
        i > limit
    }
    pub fn clear(&mut self) {
        self.request.clear();
        self.response.clear();
        self.gossipsub_message_block.clear();
        self.gossipsub_message_transaction.clear();
        self.gossipsub_message_stake.clear();
        self.gossipsub_message_peers.clear();
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Timeout {
    pub request: HashMap<IpAddr, u32>,
    pub response: HashMap<IpAddr, u32>,
}
impl Timeout {
    pub fn insert(&mut self, ip_addr: IpAddr, endpoint: Endpoint) {
        let map = match endpoint {
            Endpoint::Request => &mut self.request,
            Endpoint::Response => &mut self.response,
            _ => unimplemented!(),
        };
        map.insert(ip_addr, Utc::now().timestamp() as u32);
    }
    pub fn has(&self, ip_addr: IpAddr, endpoint: Endpoint) -> bool {
        let map = match endpoint {
            Endpoint::Request => &self.request,
            Endpoint::Response => &self.response,
            _ => unimplemented!(),
        };
        let limit = match endpoint {
            Endpoint::Request => P2P_RATELIMIT_REQUEST_TIMEOUT,
            Endpoint::Response => P2P_RATELIMIT_RESPONSE_TIMEOUT,
            _ => unimplemented!(),
        };
        let timestamp = map.get(&ip_addr).unwrap_or(&0);
        Utc::now().timestamp() as u32 - timestamp < limit
    }
}
