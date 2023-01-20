use crate::node::Node;
use crate::util;
use libp2p::gossipsub::GossipsubMessage;
use libp2p::request_response::ResponseChannel;
use libp2p::Multiaddr;
use libp2p::PeerId;
use log::debug;
use pea_block::BlockB;
use pea_core::*;
use pea_p2p::behaviour::SyncRequest;
use pea_p2p::behaviour::SyncResponse;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use std::collections::HashMap;
use std::error::Error;
use std::net::IpAddr;
pub enum Endpoint {
    Block,
    Transaction,
    Stake,
    Multiaddr,
    SyncRequest,
    SyncResponse,
}
#[derive(Debug, Default)]
pub struct Ratelimit {
    map: HashMap<IpAddr, ([usize; 6], Option<u32>)>,
}
impl Ratelimit {
    pub fn get(&self, addr: &IpAddr) -> ([usize; 6], Option<u32>) {
        *self.map.get(addr).unwrap_or(&([0; 6], None))
    }
    pub fn is_ratelimited(&self, b: &Option<u32>) -> bool {
        if let Some(timestamp) = b {
            if timestamp + RATELIMIT_DURATION > util::timestamp() {
                return true;
            }
        }
        false
    }
    pub fn add(&mut self, addr: IpAddr, endpoint: Endpoint) -> bool {
        let mut value = self.get(&addr);
        let a = &mut value.0;
        let b = &mut value.1;
        if self.is_ratelimited(b) {
            return true;
        }
        let ratelimited = match endpoint {
            Endpoint::Block => {
                a[0] += 1;
                a[0] > RATELIMIT_BLOCK
            }
            Endpoint::Transaction => {
                a[1] += 1;
                a[1] > RATELIMIT_TRANSACTION
            }
            Endpoint::Stake => {
                a[2] += 1;
                a[2] > RATELIMIT_STAKE
            }
            Endpoint::Multiaddr => {
                a[3] += 1;
                a[3] > RATELIMIT_MULTIADDR
            }
            Endpoint::SyncRequest => {
                a[4] += 1;
                a[4] > RATELIMIT_SYNC_REQUEST
            }
            Endpoint::SyncResponse => {
                a[5] += 1;
                a[5] > RATELIMIT_SYNC_RESPONSE
            }
        };
        if ratelimited {
            *b = Some(util::timestamp());
        }
        self.map.insert(addr, value);
        ratelimited
    }
    pub fn reset(&mut self) {
        for value in self.map.values_mut() {
            let a = &mut value.0;
            a[0] = a[0].saturating_sub(RATELIMIT_BLOCK);
            a[1] = a[1].saturating_sub(RATELIMIT_TRANSACTION);
            a[2] = a[2].saturating_sub(RATELIMIT_STAKE);
            a[3] = a[3].saturating_sub(RATELIMIT_MULTIADDR);
            a[4] = a[4].saturating_sub(RATELIMIT_SYNC_REQUEST);
            a[5] = a[5].saturating_sub(RATELIMIT_SYNC_RESPONSE);
        }
    }
    pub fn ratelimit(node: &mut Node, peer_id: PeerId, endpoint: Endpoint) -> Result<(), Box<dyn Error>> {
        let (multiaddr, _) = node.p2p_connections.iter().find(|x| x.1 == &peer_id).unwrap();
        let addr = pea_p2p::multiaddr::multiaddr_addr(multiaddr).expect("multiaddr to include ip");
        if node.p2p_ratelimit.add(addr, endpoint) {
            let _ = node.p2p_swarm.disconnect_peer_id(peer_id);
            return Err("ratelimited".into());
        }
        Ok(())
    }
}
pub fn gossipsub_handler(node: &mut Node, message: GossipsubMessage, propagation_source: PeerId) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            Ratelimit::ratelimit(node, propagation_source, Endpoint::Block)?;
            let block_b: BlockB = bincode::deserialize(&message.data)?;
            node.blockchain.append_block(block_b, util::timestamp())?;
        }
        "transaction" => {
            Ratelimit::ratelimit(node, propagation_source, Endpoint::Transaction)?;
            let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_transactions_push(transaction_b, util::timestamp())?;
        }
        "stake" => {
            Ratelimit::ratelimit(node, propagation_source, Endpoint::Stake)?;
            let stake_b: StakeB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_stakes_push(stake_b, util::timestamp())?;
        }
        "multiaddr" => {
            Ratelimit::ratelimit(node, propagation_source, Endpoint::Multiaddr)?;
            for multiaddr in bincode::deserialize::<Vec<Multiaddr>>(&message.data)? {
                if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip_port(&multiaddr) {
                    node.p2p_unknown.insert(multiaddr);
                }
            }
        }
        _ => {}
    };
    Ok(())
}
pub fn request_handler(node: &mut Node, peer_id: PeerId, request: SyncRequest, channel: ResponseChannel<SyncResponse>) -> Result<(), Box<dyn Error>> {
    Ratelimit::ratelimit(node, peer_id, Endpoint::SyncRequest)?;
    let height: usize = bincode::deserialize(&request.0)?;
    let mut vec = vec![];
    for i in 0..SYNC_BLOCKS_PER_TICK {
        match node.blockchain.sync_block(height + i) {
            Some(block_b) => vec.push(block_b),
            None => break,
        }
    }
    if node
        .p2p_swarm
        .behaviour_mut()
        .request_response
        .send_response(channel, SyncResponse(bincode::serialize(&vec).unwrap()))
        .is_err()
    {
        return Err("p2p request handler connection closed".into());
    };
    Ok(())
}
pub fn response_handler(node: &mut Node, peer_id: PeerId, response: SyncResponse) -> Result<(), Box<dyn Error>> {
    Ratelimit::ratelimit(node, peer_id, Endpoint::SyncResponse)?;
    let timestamp = util::timestamp();
    for block_b in bincode::deserialize::<Vec<BlockB>>(&response.0)? {
        if let Err(err) = node.blockchain.append_block(block_b, timestamp) {
            debug!("response_handler {}", err);
        }
    }
    Ok(())
}
