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
    pub fn get_dynamic(&self) -> &Dynamic {
        &self.dynamic
    }
    pub fn get_trusted(&self) -> &Trusted {
        &self.trusted
    }
    pub fn get_fork_state(
        &self,
        blockchain: &Blockchain,
        previous_hash: &types::Hash,
    ) -> Result<Dynamic, Box<dyn Error>> {
        if previous_hash == &[0; 32] {
            return Ok(Dynamic::default());
        }
        let mut hashes = vec![];
        if let Some(first) = self.dynamic.get_hashes().first() {
            let mut hash = *previous_hash;
            for _ in 0..TRUST_FORK_AFTER_BLOCKS {
                hashes.push(hash);
                if first == &hash {
                    break;
                }
                match blockchain.get_tree().get(&hash) {
                    Some(previous_hash) => hash = *previous_hash,
                    None => break,
                };
            }
            if first != &hash {
                return Err("not allowed to fork trusted chain".into());
            }
            hashes.reverse();
        }
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
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, mut hashes: Vec<types::Hash>) {
        let len = hashes.len();
        let start = if len < TRUST_FORK_AFTER_BLOCKS {
            0
        } else {
            len - TRUST_FORK_AFTER_BLOCKS
        };
        hashes.drain(start..len);
        self.trusted.load(db, &hashes);
        self.reload(db, &hashes);
    }
    pub fn reload(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &Vec<types::Hash>) {
        self.dynamic.reload(db, hashes);
    }
}
impl Default for States {
    fn default() -> Self {
        Self::new()
    }
}
