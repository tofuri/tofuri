use crate::{block::Block, p2p::MyBehaviour, stake::Stake, transaction::Transaction};
use colored::*;
use libp2p::gossipsub::GossipsubMessage;
use log::info;
use std::error::Error;
pub fn handle(
    behaviour: &mut MyBehaviour,
    message: GossipsubMessage,
) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            let block: Block = bincode::deserialize(&message.data)?;
            behaviour.blockchain.pending_blocks_push(block.clone())?;
            let hash = behaviour.blockchain.append(&block);
            info!(
                "{} {} {}",
                "Accept".green(),
                behaviour
                    .blockchain
                    .get_tree()
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
