use crate::{
    blockchain::Blockchain,
    state::{Dynamic, Trusted},
};
use colored::*;
use log::debug;
use pea_core::*;
use pea_db as db;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::{error::Error, time::Instant};
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
    pub fn dynamic_fork(&self, blockchain: &Blockchain, previous_hash: &Hash) -> Result<Dynamic, Box<dyn Error>> {
        if previous_hash == &[0; 32] {
            return Ok(Dynamic::default());
        }
        let mut hashes = vec![];
        if let Some(first) = blockchain.states.dynamic.hashes.first() {
            let mut hash = *previous_hash;
            for _ in 0..blockchain.trust_fork_after_blocks {
                hashes.push(hash);
                if first == &hash {
                    break;
                }
                match blockchain.tree.get(&hash) {
                    Some(previous_hash) => hash = *previous_hash,
                    None => break,
                };
            }
            if first != &hash && hash != [0; 32] {
                return Err("not allowed to fork trusted chain".into());
            }
            if let Some(hash) = hashes.last() {
                if hash == &[0; 32] {
                    hashes.pop();
                }
            }
            hashes.reverse();
        }
        Ok(Dynamic::from(&blockchain.db, &hashes, &self.trusted))
    }
    pub fn update(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes_1: &[Hash], trust_fork_after_blocks: usize) {
        let start = Instant::now();
        let hashes_0 = &self.dynamic.hashes;
        if hashes_0.len() == trust_fork_after_blocks {
            let block_a = db::block::get_a(db, hashes_0.first().unwrap()).unwrap();
            self.trusted.update(
                db,
                &block_a,
                match db::block::get_b(db, &block_a.previous_hash) {
                    Ok(block_b) => block_b.timestamp,
                    Err(_) => 0,
                },
                false,
            );
        }
        self.dynamic = Dynamic::from(db, hashes_1, &self.trusted);
        debug!("{} {:?}", "States update".cyan(), start.elapsed());
    }
}
impl Default for States {
    fn default() -> Self {
        Self::new()
    }
}
