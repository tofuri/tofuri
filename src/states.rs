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
    dynamic: Dynamic,
    trusted: Trusted,
}
impl States {
    pub fn new() -> States {
        States {
            dynamic: Dynamic::default(),
            trusted: Trusted::default(),
        }
    }
    pub fn get_current(&self) -> &Dynamic {
        &self.dynamic
    }
    pub fn get_current_mut(&mut self) -> &mut Dynamic {
        &mut self.dynamic
    }
    pub fn get_previous(&self) -> &Trusted {
        &self.trusted
    }
    pub fn get_previous_mut(&mut self) -> &mut Trusted {
        &mut self.trusted
    }
    pub fn get_fork_state(
        &self,
        blockchain: &Blockchain,
        previous_hash: &types::Hash,
    ) -> Result<Dynamic, Box<dyn Error>> {
        if previous_hash == &[0; 32] {
            return Ok(Dynamic::default());
        }
        if let Some(hash) = blockchain
            .get_tree()
            .get_fork_vec(self.dynamic.get_hashes(), *previous_hash)
            .first()
        {
            if self
                .dynamic
                .get_hashes()
                .iter()
                .position(|x| x == hash)
                .unwrap()
                + TRUST_FORK_AFTER_BLOCKS
                <= blockchain.get_height()
            {
                return Err("not allowed to fork trusted chain".into());
            }
        }
        let fork_vec = blockchain
            .get_tree()
            .get_fork_vec(self.trusted.get_hashes(), *previous_hash);
        let mut fork_state = Dynamic::from(&self.trusted);
        let mut previous_timestamp = match fork_vec.first() {
            Some(hash) => Self::get_previous_timestamp(blockchain.get_db(), hash),
            None => 0,
        };
        for hash in fork_vec.iter() {
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
        self.dynamic.append(
            block.clone(),
            Self::get_previous_timestamp(db, &block.previous_hash),
        );
        let hashes = self.dynamic.get_hashes();
        let len = hashes.len();
        if len > TRUST_FORK_AFTER_BLOCKS {
            let block = Block::get(db, &hashes[len - 1 - TRUST_FORK_AFTER_BLOCKS]).unwrap();
            let previous_hash = block.previous_hash;
            self.trusted
                .append(block, Self::get_previous_timestamp(db, &previous_hash));
        }
    }
    pub fn reload(&mut self, db: &DBWithThreadMode<SingleThreaded>, mut hashes: Vec<types::Hash>) {
        self.dynamic.reload(db, hashes.clone());
        let len = hashes.len();
        let start = if len < TRUST_FORK_AFTER_BLOCKS {
            0
        } else {
            len - TRUST_FORK_AFTER_BLOCKS
        };
        hashes.drain(start..len);
        self.trusted.reload(db, hashes);
    }
}
impl Default for States {
    fn default() -> Self {
        Self::new()
    }
}
