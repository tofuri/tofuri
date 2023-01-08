use crate::util;
use crate::{multiaddr, node::Node};
use libp2p::{gossipsub::GossipsubMessage, Multiaddr, PeerId};
use pea_block::BlockB;
use pea_core::*;
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
    map: HashMap<IpAddr, ([usize; 5], Option<u32>)>,
}
impl Ratelimit {
    pub fn get(&self, ip: &IpAddr) -> ([usize; 5], Option<u32>) {
        match self.map.get(ip) {
            Some(x) => *x,
            None => ([0; 5], None),
        }
    }
    pub fn add(&mut self, ip: IpAddr, topic: Topic) -> bool {
        let mut value = self.get(&ip);
        let a = &mut value.0;
        let b = &mut value.1;
        if let Some(timestamp) = b {
            if *timestamp + RATELIMIT_DURATION > util::timestamp() {
                return true;
            }
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
        self.map.insert(ip, value);
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
