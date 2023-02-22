use crate::state::Dynamic;
use crate::state::Trusted;
use colored::*;
use log::debug;
use pea_core::*;
use pea_db as db;
use pea_tree::Tree;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use std::error::Error;
use std::time::Instant;
#[derive(Default, Debug, Clone)]
pub struct States {
    pub dynamic: Dynamic,
    pub trusted: Trusted,
}
impl States {
    pub fn dynamic_fork(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        tree: &Tree,
        trust_fork_after_blocks: usize,
        previous_hash: &Hash,
    ) -> Result<Dynamic, Box<dyn Error>> {
        if previous_hash == &[0; 32] {
            return Ok(Dynamic::default());
        }
        let mut hashes = vec![];
        if let Some(first) = self.dynamic.hashes.first() {
            let mut hash = *previous_hash;
            for _ in 0..trust_fork_after_blocks {
                hashes.push(hash);
                if first == &hash {
                    break;
                }
                match tree.get(&hash) {
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
        Ok(Dynamic::from(db, &hashes, &self.trusted))
    }
    pub fn update(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes_1: &[Hash], trust_fork_after_blocks: usize) {
        let start = Instant::now();
        let hashes_0 = &self.dynamic.hashes;
        if hashes_0.len() == trust_fork_after_blocks {
            let block_a = db::block::get_a(db, hashes_0.first().unwrap()).unwrap();
            self.trusted.append_block(
                &block_a,
                match db::block::get_b(db, &block_a.previous_hash) {
                    Ok(block_b) => block_b.timestamp,
                    Err(_) => 0,
                },
            );
        }
        self.dynamic = Dynamic::from(db, hashes_1, &self.trusted);
        debug!("{} {:?}", "States update".cyan(), start.elapsed());
    }
}
