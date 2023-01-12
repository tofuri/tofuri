use crate::node::Node;
use crate::util;
use libp2p::gossipsub::GossipsubMessage;
use libp2p::request_response::ResponseChannel;
use libp2p::Multiaddr;
use libp2p::PeerId;
use pea_block::BlockB;
use pea_core::*;
use pea_p2p::behaviour::FileRequest;
use pea_p2p::behaviour::FileResponse;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use std::collections::HashMap;
use std::error::Error;
use std::net::IpAddr;
pub enum Topic {
    Block,
    Transaction,
    Stake,
    Multiaddr,
    Blocks,
}
#[derive(Debug, Default)]
pub struct Ratelimit {
    map: HashMap<IpAddr, ([usize; 5], Option<u32>)>,
}
impl Ratelimit {
    pub fn get(&self, addr: &IpAddr) -> ([usize; 5], Option<u32>) {
        match self.map.get(addr) {
            Some(x) => *x,
            None => ([0; 5], None),
        }
    }
    pub fn is_ratelimited(&self, b: &Option<u32>) -> bool {
        if let Some(timestamp) = b {
            if timestamp + RATELIMIT_DURATION > util::timestamp() {
                return true;
            }
        }
        false
    }
    pub fn add(&mut self, addr: IpAddr, topic: Topic) -> bool {
        let mut value = self.get(&addr);
        let a = &mut value.0;
        let b = &mut value.1;
        if self.is_ratelimited(b) {
            return true;
        }
        let ratelimited = match topic {
            Topic::Block => {
                a[0] += 1;
                a[0] > RATELIMIT_TOPIC_BLOCK
            }
            Topic::Blocks => {
                a[1] += 1;
                a[1] > RATELIMIT_TOPIC_BLOCKS
            }
            Topic::Transaction => {
                a[2] += 1;
                a[2] > RATELIMIT_TOPIC_TRANSACTION
            }
            Topic::Stake => {
                a[3] += 1;
                a[3] > RATELIMIT_TOPIC_STAKE
            }
            Topic::Multiaddr => {
                a[4] += 1;
                a[4] > RATELIMIT_TOPIC_MULTIADDR
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
            a[0] = a[0].saturating_sub(RATELIMIT_TOPIC_BLOCK);
            a[1] = a[1].saturating_sub(RATELIMIT_TOPIC_BLOCKS);
            a[2] = a[2].saturating_sub(RATELIMIT_TOPIC_TRANSACTION);
            a[3] = a[3].saturating_sub(RATELIMIT_TOPIC_STAKE);
            a[4] = a[4].saturating_sub(RATELIMIT_TOPIC_MULTIADDR);
        }
    }
}
pub fn ratelimit(node: &mut Node, addr: IpAddr, propagation_source: PeerId, topic: Topic) -> Result<(), Box<dyn Error>> {
    if node.p2p_ratelimit.add(addr, topic) {
        let _ = node.p2p_swarm.disconnect_peer_id(propagation_source);
        return Err("ratelimited".into());
    }
    Ok(())
}
pub fn request_handler(node: &mut Node, peer_id: PeerId, request: FileRequest, channel: ResponseChannel<FileResponse>) -> Result<(), Box<dyn Error>> {
    println!("{:?}", request);
    Ok(())
}
pub fn response_handler(node: &mut Node, peer_id: PeerId, response: FileResponse) -> Result<(), Box<dyn Error>> {
    Ok(())
}
pub fn gossipsub_handler(node: &mut Node, message: GossipsubMessage, propagation_source: PeerId) -> Result<(), Box<dyn Error>> {
    let (multiaddr, _) = node.p2p_connections.iter().find(|x| x.1 == &propagation_source).unwrap();
    let addr = pea_p2p::multiaddr::multiaddr_addr(multiaddr).expect("multiaddr to include ip");
    match message.topic.as_str() {
        "block" => {
            ratelimit(node, addr, propagation_source, Topic::Block)?;
            let block_b: BlockB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_blocks_push(block_b, util::timestamp())?;
        }
        "blocks" => {
            ratelimit(node, addr, propagation_source, Topic::Blocks)?;
            for block_b in bincode::deserialize::<Vec<BlockB>>(&message.data)? {
                ratelimit(node, addr, propagation_source, Topic::Block)?;
                node.blockchain.pending_blocks_push(block_b, util::timestamp())?;
            }
        }
        "transaction" => {
            ratelimit(node, addr, propagation_source, Topic::Transaction)?;
            let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_transactions_push(transaction_b, util::timestamp())?;
        }
        "stake" => {
            ratelimit(node, addr, propagation_source, Topic::Stake)?;
            let stake_b: StakeB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_stakes_push(stake_b, util::timestamp())?;
        }
        "multiaddr" => {
            ratelimit(node, addr, propagation_source, Topic::Multiaddr)?;
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
