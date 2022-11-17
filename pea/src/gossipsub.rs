use crate::{multiaddr, node::Node};
use colored::*;
use libp2p::{gossipsub::GossipsubMessage, Multiaddr};
use log::info;
use pea_block::Block;
use pea_stake::Stake;
use pea_transaction::Transaction;
use std::error::Error;
pub fn handler(node: &mut Node, message: GossipsubMessage) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            block(node, &message.data)?;
        }
        "block sync" => {
            block(node, &message.data)?;
        }
        "stake" => {
            let stake: Stake = bincode::deserialize(&message.data)?;
            node.blockchain.pending_stakes_push(stake)?;
        }
        "transaction" => {
            let transaction: Transaction = bincode::deserialize(&message.data)?;
            node.blockchain.pending_transactions_push(transaction)?;
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
fn block(node: &mut Node, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let block: Block = bincode::deserialize(bytes)?;
    node.blockchain.pending_blocks_push(block.clone())?;
    let hash = node.blockchain.block_accept(&block);
    info!(
        "{} {} {}",
        "Accept".green(),
        node.blockchain.tree.height(&block.previous_hash).to_string().yellow(),
        hex::encode(hash)
    );
    Ok(())
}
