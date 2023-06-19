use crate::Error;
use crate::Stable;
use crate::Unstable;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use serde::Deserialize;
use serde::Serialize;
use tofuri_tree::Tree;
use tofuri_tree::GENESIS_BLOCK_PREVIOUS_HASH;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Manager {
    pub stable: Stable,
    pub unstable: Unstable,
}
impl Manager {
    pub fn unstable(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        tree: &Tree,
        trust_fork_after_blocks: usize,
        previous_hash: &[u8; 32],
    ) -> Result<Unstable, Error> {
        if previous_hash == &GENESIS_BLOCK_PREVIOUS_HASH {
            let unstable = Unstable::default();
            return Ok(unstable);
        }
        let first = self.unstable.hashes.first().unwrap();
        let mut hashes = vec![];
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
        if first != &hash && hash != GENESIS_BLOCK_PREVIOUS_HASH {
            return Err(Error::NotAllowedToForkStableChain);
        }
        if let Some(hash) = hashes.last() {
            if hash == &GENESIS_BLOCK_PREVIOUS_HASH {
                hashes.pop();
            }
        }
        hashes.reverse();
        let unstable = Unstable::from(db, &hashes, &self.stable);
        Ok(unstable)
    }
    pub fn update(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        hashes_1: &[[u8; 32]],
        trust_fork_after_blocks: usize,
    ) {
        let hashes_0 = &self.unstable.hashes;
        if hashes_0.len() == trust_fork_after_blocks {
            let block_a = tofuri_db::block::get(db, hashes_0.first().unwrap()).unwrap();
            self.stable.append_block(
                &block_a,
                match tofuri_db::block::get(db, &block_a.previous_hash) {
                    Ok(block_b) => block_b.timestamp,
                    Err(_) => 0,
                },
            );
        }
        self.unstable = Unstable::from(db, hashes_1, &self.stable);
    }
}
