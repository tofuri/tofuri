use std::collections::HashMap;
use std::net::IpAddr;
use tofuri_core::*;
pub enum Endpoint {
    RequestResponse,
    GossipsubMessageBlock,
    GossipsubMessageTransaction,
    GossipsubMessageStake,
    GossipsubMessagePeers,
}
#[derive(Debug, Clone, Default)]
pub struct Ratelimit {
    pub counter: Counter,
    pub timeout: Timeout,
}
#[derive(Debug, Clone, Default)]
pub struct Counter {
    pub request_response: HashMap<IpAddr, usize>,
    pub gossipsub_message_block: HashMap<IpAddr, usize>,
    pub gossipsub_message_transaction: HashMap<IpAddr, usize>,
    pub gossipsub_message_stake: HashMap<IpAddr, usize>,
    pub gossipsub_message_peers: HashMap<IpAddr, usize>,
}
impl Counter {
    pub fn add(&mut self, ip_addr: IpAddr, endpoint: &Endpoint) -> bool {
        let (hash_map, limit) = match endpoint {
            Endpoint::RequestResponse => {
                (&mut self.request_response, P2P_RATELIMIT_REQUEST_RESPONSE)
            }
            Endpoint::GossipsubMessageBlock => (
                &mut self.gossipsub_message_block,
                P2P_RATELIMIT_GOSSIPSUB_MESSAGE_BLOCK,
            ),
            Endpoint::GossipsubMessageTransaction => (
                &mut self.gossipsub_message_transaction,
                P2P_RATELIMIT_GOSSIPSUB_MESSAGE_TRANSACTION,
            ),
            Endpoint::GossipsubMessageStake => (
                &mut self.gossipsub_message_stake,
                P2P_RATELIMIT_GOSSIPSUB_MESSAGE_STAKE,
            ),
            Endpoint::GossipsubMessagePeers => (
                &mut self.gossipsub_message_peers,
                P2P_RATELIMIT_GOSSIPSUB_MESSAGE_PEERS,
            ),
        };
        let mut counter = *hash_map.get(&ip_addr).unwrap_or(&0);
        counter += 1;
        hash_map.insert(ip_addr, counter);
        counter > limit
    }
    pub fn clear(&mut self) {
        self.request_response.clear();
        self.gossipsub_message_block.clear();
        self.gossipsub_message_transaction.clear();
        self.gossipsub_message_stake.clear();
        self.gossipsub_message_peers.clear();
    }
}
#[derive(Debug, Clone, Default)]
pub struct Timeout {
    pub request_response: HashMap<IpAddr, u32>,
}
impl Timeout {
    pub fn insert(&mut self, ip_addr: IpAddr, endpoint: Endpoint) {
        let hash_map = match endpoint {
            Endpoint::RequestResponse => &mut self.request_response,
            _ => return,
        };
        hash_map.insert(ip_addr, tofuri_util::timestamp());
    }
    pub fn has(&self, ip_addr: IpAddr, endpoint: Endpoint) -> bool {
        let (hash_map, limit) = match endpoint {
            Endpoint::RequestResponse => (
                &self.request_response,
                P2P_RATELIMIT_REQUEST_RESPONSE_TIMEOUT,
            ),
            _ => return false,
        };
        let timestamp = hash_map.get(&ip_addr).unwrap_or(&0);
        tofuri_util::timestamp() - timestamp < limit
    }
}
