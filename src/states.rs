use crate::{
    block::Block,
    blockchain::Blockchain,
    constants::TRUST_FORK_AFTER_BLOCKS,
    state::{Dynamic, Trusted},
    types,
};
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::error::Error;
#[derive(Debug)]
pub struct States {
    pub dynamic: Dynamic,
    pub trusted: Trusted,
}
impl States {
    pub fn new() -> States {
        States {
            dynamic: Dynamic::default(),
            trusted: Trusted::default(),
        }
    }
    pub fn get_fork_state(
        &self,
        blockchain: &Blockchain,
        previous_hash: &types::Hash,
    ) -> Result<Dynamic, Box<dyn Error>> {
        if previous_hash == &[0; 32] {
            return Ok(Dynamic::default());
        }
        let hashes = blockchain
            .get_tree()
            .get_hashes_dynamic(&blockchain.get_states().dynamic, previous_hash)?;
        let mut fork_state = Dynamic::from(&self.trusted);
        let mut previous_timestamp = match hashes.first() {
            Some(hash) => Self::get_previous_timestamp(blockchain.get_db(), hash),
            None => 0,
        };
        for hash in hashes.iter() {
            println!("{}", hex::encode(hash));
            let block = Block::get(blockchain.get_db(), hash).unwrap();
            let t = block.timestamp;
            fork_state.append(block, previous_timestamp);
            previous_timestamp = t;
        }
        Ok(fork_state)
    }
    fn get_previous_timestamp(
        db: &DBWithThreadMode<SingleThreaded>,
        previous_hash: &types::Hash,
    ) -> types::Timestamp {
        match Block::get(db, previous_hash) {
            Ok(block) => block.timestamp,
            Err(_) => 0,
        }
    }
    pub fn append(&mut self, db: &DBWithThreadMode<SingleThreaded>, block: &Block) {
        if let Some(hash) = self.dynamic.append(
            block.clone(),
            Self::get_previous_timestamp(db, &block.previous_hash),
        ) {
            let block = Block::get(db, &hash).unwrap();
            let previous_hash = block.previous_hash;
            self.trusted
                .append(block, Self::get_previous_timestamp(db, &previous_hash));
        }
    }
}
impl Default for States {
    fn default() -> Self {
        Self::new()
    }
}
