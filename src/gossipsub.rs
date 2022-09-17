use crate::{
    block::Block,
    constants::{BLOCKS_PER_SECOND_THRESHOLD, SYNC_BLOCKS},
    p2p::MyBehaviour,
    stake::Stake,
    sync::Sync,
    transaction::Transaction,
};
use libp2p::gossipsub::{GossipsubMessage, IdentTopic};
use std::error::Error;
pub fn handle(
    behaviour: &mut MyBehaviour,
    message: GossipsubMessage,
) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            let block: Block = bincode::deserialize(&message.data)?;
            let previous_hash = block.previous_hash;
            behaviour
                .validator
                .blockchain
                .try_add_block(&behaviour.validator.db, block)?;
            if behaviour.validator.synchronizer.bps >= BLOCKS_PER_SECOND_THRESHOLD {
                // accept block early for faster synchronization
                behaviour
                    .validator
                    .blockchain
                    .accept_block(&behaviour.validator.db, false)?
            }
            if behaviour
                .validator
                .blockchain
                .get_latest_block()
                .previous_hash
                == previous_hash
            {
                behaviour.validator.synchronizer.new += 1;
            }
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
        "sync" => {
            let sync: Sync = bincode::deserialize(&message.data)?;
            for i in sync.height..=sync.height + SYNC_BLOCKS {
                if i >= behaviour.validator.blockchain.get_hashes().len() {
                    break;
                }
                let hash = behaviour.validator.blockchain.get_hashes().get(i).unwrap();
                if behaviour.gossipsub.all_peers().count() > 0 {
                    behaviour.gossipsub.publish(
                        IdentTopic::new("block"),
                        bincode::serialize(&Block::get(&behaviour.validator.db, hash)?)?,
                    )?;
                }
            }
        }
        _ => {}
    };
    Ok(())
}
