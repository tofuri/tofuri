use crate::util;
use crate::{multiaddr, node::Node};
use libp2p::PeerId;
use libp2p::{gossipsub::GossipsubMessage, Multiaddr};
use pea_block::BlockB;
use pea_core::RATELIMIT;
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
    map: HashMap<PeerId, Score>,
}
impl Ratelimit {
    pub fn get(&self, peer_id: &PeerId) -> Score {
        match self.map.get(peer_id) {
            Some(x) => *x,
            None => Score::default(),
        }
    }
    pub fn add(&mut self, peer_id: PeerId) -> bool {
        let mut score = self.get(&peer_id);
        score.new += 1.0;
        self.map.insert(peer_id, score);
        if score.new >= RATELIMIT {
            return true;
        }
        if score.avg >= RATELIMIT {
            return true;
        }
        false
    }
    pub fn update(&mut self) {
        for score in self.map.values_mut() {
            score.avg += score.new;
            score.avg /= 2.0;
            score.new = 0.0;
        }
    }
}
