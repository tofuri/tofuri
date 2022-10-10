use crate::{block::MetadataLean, constants::TRUST_FORK_AFTER_BLOCKS, db, types};
use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
type Branch = (types::Hash, types::Height, types::Timestamp);
pub struct Tree {
    branches: Vec<Branch>,
    hashes: HashMap<types::Hash, types::Hash>,
}
impl Tree {
    pub fn new() -> Tree {
        Tree {
            branches: vec![],
            hashes: HashMap::new(),
        }
    }
    pub fn main(&self) -> Option<&Branch> {
        self.branches.first()
    }
    pub fn size(&self) -> usize {
        self.hashes.len()
    }
    pub fn hashes(&self) -> (Vec<types::Hash>, Vec<types::Hash>) {
        let mut trusted = vec![];
        if let Some(main) = self.main() {
            let mut hash = main.0;
            loop {
                trusted.push(hash);
                match self.get(&hash) {
                    Some(previous_hash) => hash = *previous_hash,
                    None => break,
                };
            }
        }
        if let Some(hash) = trusted.last() {
            if hash != &[0; 32] {
                panic!("broken chain")
            }
            trusted.pop();
        }
        trusted.reverse();
        let len = trusted.len();
        let start = if len < TRUST_FORK_AFTER_BLOCKS {
            0
        } else {
            len - TRUST_FORK_AFTER_BLOCKS
        };
        let dynamic = trusted.drain(start..len).collect();
        (trusted, dynamic)
    }
    pub fn hashes_dynamic(&self) -> Vec<types::Hash> {
        let mut vec = vec![];
        if let Some(main) = self.main() {
            let mut hash = main.0;
            for _ in 0..TRUST_FORK_AFTER_BLOCKS {
                vec.push(hash);
                match self.get(&hash) {
                    Some(previous_hash) => hash = *previous_hash,
                    None => break,
                };
            }
        }
        if let Some(hash) = vec.last() {
            if hash == &[0; 32] {
                vec.pop();
            }
        }
        vec.reverse();
        vec
    }
    pub fn get(&self, hash: &types::Hash) -> Option<&types::Hash> {
        self.hashes.get(hash)
    }
    pub fn insert(
        &mut self,
        hash: types::Hash,
        previous_hash: types::Hash,
        timestamp: types::Timestamp,
    ) -> Option<bool> {
        if self.hashes.insert(hash, previous_hash).is_some() {
            return None;
        }
        if let Some(index) = self
            .branches
            .iter()
            .position(|(hash, _, _)| hash == &previous_hash)
        {
            // extend branch
            self.branches[index] = (hash, self.branches[index].1 + 1, timestamp);
            Some(false)
        } else {
            // new branch
            self.branches
                .push((hash, self.height(&previous_hash), timestamp));
            Some(true)
        }
    }
    pub fn sort_branches(&mut self) {
        self.branches.sort_by(|a, b| match b.1.cmp(&a.1) {
            Ordering::Equal => {
                // let a_block = Block::get()
                a.2.cmp(&b.2)
            }
            x => x,
        });
    }
    pub fn height(&self, previous_hash: &types::Hash) -> types::Height {
        let mut hash = previous_hash;
        let mut height = 0;
        loop {
            match self.hashes.get(hash) {
                Some(previous_hash) => {
                    hash = previous_hash;
                    height += 1;
                }
                None => {
                    if hash != &[0; 32] {
                        panic!("broken chain")
                    }
                    break;
                }
            };
        }
        height
    }
    pub fn reload(&mut self, db: &DBWithThreadMode<SingleThreaded>) {
        self.clear();
        let mut hashes: HashMap<types::Hash, (Vec<types::Hash>, types::Timestamp)> = HashMap::new();
        for res in db.iterator_cf(db::blocks(db), IteratorMode::Start) {
            let (hash, bytes) = res.unwrap();
            let hash = hash.to_vec().try_into().unwrap();
            let block: MetadataLean = bincode::deserialize(&bytes).unwrap();
            match hashes.get(&block.previous_hash) {
                Some((vec, _)) => {
                    let mut vec = vec.clone();
                    vec.push(hash);
                    hashes.insert(block.previous_hash, (vec, block.timestamp));
                }
                None => {
                    hashes.insert(block.previous_hash, (vec![hash], block.timestamp));
                }
            };
        }
        if hashes.is_empty() {
            return;
        }
        let previous_hash = [0; 32];
        let (_, (vec, timestamp)) = hashes.iter().find(|(&x, _)| x == previous_hash).unwrap();
        fn recurse(
            tree: &mut Tree,
            hashes: &HashMap<types::Hash, (Vec<types::Hash>, types::Timestamp)>,
            previous_hash: types::Hash,
            vec: &Vec<types::Hash>,
            timestamp: types::Timestamp,
        ) {
            for hash in vec {
                tree.insert(*hash, previous_hash, timestamp);
                if let Some((vec, timestamp)) = hashes.get(hash) {
                    recurse(tree, hashes, *hash, vec, *timestamp);
                };
            }
        }
        recurse(self, &hashes, previous_hash, vec, *timestamp);
        self.sort_branches();
    }
    pub fn clear(&mut self) {
        self.branches.clear();
        self.hashes.clear();
    }
}
impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #![allow(dead_code)]
        #[derive(Debug)]
        struct Tree {
            branches: Vec<(String, types::Height, types::Timestamp)>,
            hashes: HashMap<String, String>,
        }
        write!(
            f,
            "{:?}",
            Tree {
                branches: self
                    .branches
                    .iter()
                    .map(|(hash, height, timestamp)| (hex::encode(hash), *height, *timestamp))
                    .collect(),
                hashes: self
                    .hashes
                    .iter()
                    .map(|(hash, previous_hash)| (hex::encode(hash), hex::encode(previous_hash)))
                    .collect(),
            }
        )
    }
}
impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}
