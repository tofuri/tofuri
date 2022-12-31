use crate::{multiaddr, node::Node};
use libp2p::{gossipsub::GossipsubMessage, Multiaddr};
use pea_block::BlockB;
use pea_stake::StakeC;
use pea_transaction::TransactionC;
use std::error::Error;
pub fn handler(node: &mut Node, message: GossipsubMessage) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            let block_b: BlockB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_blocks_push(block_b, node.time.timestamp_secs())?;
        }
        "blocks" => {
            for block_b in bincode::deserialize::<Vec<BlockB>>(&message.data)? {
                node.blockchain.pending_blocks_push(block_b, node.time.timestamp_secs())?;
            }
        }
        "stake" => {
            let stake_c: StakeC = bincode::deserialize(&message.data)?;
            node.blockchain.pending_stakes_push(stake_c, node.time.timestamp_secs())?;
        }
        "transaction" => {
            let transaction_c: TransactionC = bincode::deserialize(&message.data)?;
            node.blockchain.pending_transactions_push(transaction_c, node.time.timestamp_secs())?;
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
