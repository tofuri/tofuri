use crate::util;
use crate::{multiaddr, node::Node};
use libp2p::{gossipsub::GossipsubMessage, Multiaddr, PeerId};
use pea_block::BlockB;
use pea_core::RATELIMIT;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use std::collections::HashMap;
use std::error::Error;
use std::net::IpAddr;
pub fn handler(node: &mut Node, message: GossipsubMessage, propagation_source: PeerId) -> Result<(), Box<dyn Error>> {
    let (multiaddr, _) = node.p2p_connections.iter().find(|x| x.1 == &propagation_source).unwrap();
    let ip = multiaddr::ip(multiaddr).expect("multiaddr with ip");
    match message.topic.as_str() {
        "block" => {
            ratelimit(node, ip, propagation_source, Topic::Block)?;
            let block_b: BlockB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_blocks_push(block_b, util::timestamp())?;
        }
        "blocks" => {
            ratelimit(node, ip, propagation_source, Topic::Blocks)?;
            for block_b in bincode::deserialize::<Vec<BlockB>>(&message.data)? {
                node.blockchain.pending_blocks_push(block_b, util::timestamp())?;
            }
        }
        "transaction" => {
            ratelimit(node, ip, propagation_source, Topic::Transaction)?;
            let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_transactions_push(transaction_b, util::timestamp())?;
        }
        "stake" => {
            ratelimit(node, ip, propagation_source, Topic::Stake)?;
            let stake_b: StakeB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_stakes_push(stake_b, util::timestamp())?;
        }
        "multiaddr" => {
            ratelimit(node, ip, propagation_source, Topic::Multiaddr)?;
            for multiaddr in bincode::deserialize::<Vec<Multiaddr>>(&message.data)? {
                if let Some(multiaddr) = multiaddr::filter_ip_port(&multiaddr) {
                    node.p2p_unknown.insert(multiaddr);
                }
            }
        }
        _ => {}
    };
    Ok(())
}
pub fn ratelimit(node: &mut Node, ip: IpAddr, propagation_source: PeerId, topic: Topic) -> Result<(), Box<dyn Error>> {
    if node.p2p_ratelimit.add(ip, topic) {
        node.p2p_swarm.ban_peer_id(propagation_source);
        let _ = node.p2p_swarm.disconnect_peer_id(propagation_source);
        return Err("ratelimited".into());
    }
    Ok(())
}
pub enum Topic {
    Block,
    Blocks,
    Transaction,
    Stake,
    Multiaddr,
}
#[derive(Debug, Default)]
pub struct Ratelimit {
    map: HashMap<IpAddr, [usize; 5]>,
}
impl Ratelimit {
    pub fn get(&self, ip: &IpAddr) -> [usize; 5] {
        match self.map.get(ip) {
            Some(x) => *x,
            None => [0; 5],
        }
    }
    pub fn add(&mut self, ip: IpAddr, topic: Topic) -> bool {
        let mut value = self.get(&ip);
        match topic {
            Topic::Block => {
                value[0] += 1;
                value[0] > RATELIMIT
            }
            Topic::Blocks => {
                value[1] += 1;
                value[1] > RATELIMIT
            }
            Topic::Transaction => {
                value[2] += 1;
                value[2] > RATELIMIT
            }
            Topic::Stake => {
                value[3] += 1;
                value[3] > RATELIMIT
            }
            Topic::Multiaddr => {
                value[4] += 1;
                value[4] > RATELIMIT
            }
        }
    }
    pub fn reset(&mut self) {
        for value in self.map.values_mut() {
            value[0] = value[0].saturating_sub(RATELIMIT);
            value[1] = value[1].saturating_sub(RATELIMIT);
            value[2] = value[2].saturating_sub(RATELIMIT);
            value[3] = value[3].saturating_sub(RATELIMIT);
            value[4] = value[4].saturating_sub(RATELIMIT);
        }
    }
}
