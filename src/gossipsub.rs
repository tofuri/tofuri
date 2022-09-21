use crate::{block::Block, p2p::MyBehaviour, stake::Stake, transaction::Transaction};
use libp2p::gossipsub::GossipsubMessage;
use std::error::Error;
pub fn handle(
    behaviour: &mut MyBehaviour,
    message: GossipsubMessage,
) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            let block: Block = bincode::deserialize(&message.data)?;
            behaviour.blockchain.pending_blocks_push(block)?;
            behaviour.blockchain.synchronizer.new += 1;
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
        "ip" => {}
        _ => {}
    };
    Ok(())
}
