use crate::node::Node;
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
            let multiaddr: Multiaddr = bincode::deserialize(&message.data)?;
            if let Some(multiaddr) = Node::multiaddr_ip(multiaddr) {
                node.new_multiaddrs.insert(multiaddr.clone());
                info!("{} {} {}", "Multiaddr".cyan(), node.new_multiaddrs.len().to_string().yellow(), multiaddr.to_string().magenta());
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
    info!("{} {} {}", "Accept".green(), node.blockchain.tree.height(&block.previous_hash).to_string().yellow(), hex::encode(hash));
    Ok(())
}
