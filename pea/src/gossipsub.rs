use crate::{multiaddr, node::Node};
use libp2p::{gossipsub::GossipsubMessage, Multiaddr};
use pea_block::BlockB;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use std::error::Error;
pub fn handler(node: &mut Node, message: GossipsubMessage) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            let block_b: BlockB = bincode::deserialize(&message.data)?;
            node.blockchain.add_block(block_b)?;
        }
        "blocks" => {
            for block_b in bincode::deserialize::<Vec<BlockB>>(&message.data)? {
                node.blockchain.add_block(block_b)?;
            }
        }
        "stake" => {
            let stake_b: StakeB = bincode::deserialize(&message.data)?;
            node.blockchain.add_stake(stake_b, node.time.timestamp_secs())?;
        }
        "transaction" => {
            let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
            node.blockchain.add_transaction(transaction_b, node.time.timestamp_secs())?;
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
