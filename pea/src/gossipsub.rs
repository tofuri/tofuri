use crate::{multiaddr, node::Node};
use libp2p::{gossipsub::GossipsubMessage, Multiaddr};
use pea_block::Block;
use pea_stake::Stake;
use pea_transaction::Transaction;
use std::error::Error;
pub fn handler(node: &mut Node, message: GossipsubMessage) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            let block: Block = bincode::deserialize(&message.data)?;
            if node.blockchain.pending_blocks.len() < node.blockchain.pending_blocks_limit {
                node.blockchain.pending_blocks.push(block);
            }
        }
        "blocks" => {
            for block in bincode::deserialize::<Vec<Block>>(&message.data)? {
                if node.blockchain.pending_blocks.len() < node.blockchain.pending_blocks_limit {
                    node.blockchain.pending_blocks.push(block);
                }
            }
        }
        "stake" => {
            let stake: Stake = bincode::deserialize(&message.data)?;
            node.blockchain.try_add_stake(stake, node.time.timestamp_secs())?;
        }
        "transaction" => {
            let transaction: Transaction = bincode::deserialize(&message.data)?;
            node.blockchain.try_add_transaction(transaction, node.time.timestamp_secs())?;
        }
        "multiaddr" => {
            for multiaddr in bincode::deserialize::<Vec<Multiaddr>>(&message.data)? {
                if let Some(multiaddr) = multiaddr::filter_ip_port(&multiaddr) {
                    node.unknown.insert(multiaddr);
                }
            }
        }
        _ => {}
    };
    Ok(())
}
