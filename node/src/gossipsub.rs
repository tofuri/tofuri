use crate::p2p::MyBehaviour;
use colored::*;
use libp2p::gossipsub::GossipsubMessage;
use log::info;
use pea_core::{block::Block, stake::Stake, transaction::Transaction};
use std::error::Error;
pub fn handler(
    behaviour: &mut MyBehaviour,
    message: GossipsubMessage,
) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            let block: Block = bincode::deserialize(&message.data)?;
            behaviour.blockchain.pending_blocks_push(block.clone())?;
            let hash = behaviour.blockchain.block_accept(&block);
            info!(
                "{} {} {}",
                "Accept".green(),
                behaviour
                    .blockchain
                    .tree
                    .height(&block.previous_hash)
                    .to_string()
                    .yellow(),
                hex::encode(hash)
            );
        }
        "stake" => {
            let stake: Stake = bincode::deserialize(&message.data)?;
            behaviour.blockchain.pending_stakes_push(stake)?;
        }
        "transaction" => {
            let transaction: Transaction = bincode::deserialize(&message.data)?;
            behaviour
                .blockchain
                .pending_transactions_push(transaction)?;
        }
        _ => {}
    };
    Ok(())
}
