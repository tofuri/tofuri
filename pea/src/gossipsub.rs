use crate::util;
use crate::{multiaddr, node::Node};
use libp2p::{gossipsub::GossipsubMessage, Multiaddr};
use pea_block::BlockB;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use std::collections::HashMap;
use std::error::Error;
pub fn handler(node: &mut Node, message: GossipsubMessage) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            let block_b: BlockB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_blocks_push(block_b, util::timestamp())?;
        }
        "blocks" => {
            for block_b in bincode::deserialize::<Vec<BlockB>>(&message.data)? {
                node.blockchain.pending_blocks_push(block_b, util::timestamp())?;
            }
        }
        "stake" => {
            let stake_b: StakeB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_stakes_push(stake_b, util::timestamp())?;
        }
        "transaction" => {
            let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_transactions_push(transaction_b, util::timestamp())?;
        }
        "multiaddr" => {
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
#[derive(Debug, Default, Clone, Copy)]
pub struct Score {
    pub new: f32,
    pub avg: f32,
}
#[derive(Debug, Default)]
pub struct Ratelimit {
    map: HashMap<Multiaddr, Score>,
}
impl Ratelimit {
    pub fn add(&mut self, multiaddr: Multiaddr) -> Score {
        let mut score = match self.map.get(&multiaddr) {
            Some(x) => *x,
            None => Score::default(),
        };
        score.new += 1.0;
        self.map.insert(multiaddr, score);
        score
    }
    pub fn avg(&mut self) {
        for score in self.map.values_mut() {
            score.avg += score.new;
            score.avg /= 2.0;
            score.new = 0.0;
        }
    }
}
