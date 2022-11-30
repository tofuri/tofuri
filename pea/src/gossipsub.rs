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
            node.blockchain.try_add_block(block.clone())?;
            node.blockchain.accept_block(&block, false);
        }
        "blocks" => {
            let vec: Vec<Block> = bincode::deserialize(&message.data)?;
            for block in vec {
                if node.blockchain.try_add_block(block.clone()).is_err() {
                    continue;
                }
                node.blockchain.accept_block(&block, false);
            }
        }
        "stake" => {
            let stake: Stake = bincode::deserialize(&message.data)?;
            node.blockchain.try_add_stake(stake)?;
        }
        "transaction" => {
            let transaction: Transaction = bincode::deserialize(&message.data)?;
            node.blockchain.try_add_transaction(transaction)?;
        }
        "multiaddr" => {
            let vec: Vec<Multiaddr> = bincode::deserialize(&message.data)?;
            for multiaddr in vec {
                if let Some(multiaddr) = multiaddr::filter_ip_port(&multiaddr) {
                    node.unknown.insert(multiaddr);
                }
            }
        }
        _ => {}
    };
    Ok(())
}
