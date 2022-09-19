use crate::{
    block::Block, p2p::MyBehaviour, stake::Stake,
    transaction::Transaction,
};
use libp2p::gossipsub::GossipsubMessage;
use std::error::Error;
pub fn handle(
    behaviour: &mut MyBehaviour,
    message: GossipsubMessage,
) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            let block: Block = bincode::deserialize(&message.data)?;
            behaviour.validator.blockchain.try_accept_block(&behaviour.validator.db, block)?;
            // behaviour
                // .validator
                // .blockchain
                // .try_add_block(&behaviour.validator.db, block)?;
            // if behaviour.validator.synchronizer.bps >= BLOCKS_PER_SECOND_THRESHOLD {
                // // accept block early for faster synchronization
                // behaviour
                    // .validator
                    // .blockchain
                    // .accept_block(&behaviour.validator.db)?;
            // }
            behaviour.validator.synchronizer.new += 1;
        }
        "stake" => {
            let stake: Stake = bincode::deserialize(&message.data)?;
            behaviour
                .validator
                .blockchain
                .try_add_stake(&behaviour.validator.db, stake)?;
        }
        "transaction" => {
            let transaction: Transaction = bincode::deserialize(&message.data)?;
            behaviour
                .validator
                .blockchain
                .try_add_transaction(&behaviour.validator.db, transaction)?;
        }
        "ip" => {}
        _ => {}
    };
    Ok(())
}
