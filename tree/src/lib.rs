use pea_core::types;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
type Branch = (types::Hash, usize, u32);
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
    pub fn hashes(&self, trust_fork_after_blocks: usize) -> (Vec<types::Hash>, Vec<types::Hash>) {
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
        let start = if len < trust_fork_after_blocks { 0 } else { len - trust_fork_after_blocks };
        let dynamic = trusted.drain(start..len).collect();
        (trusted, dynamic)
    }
    pub fn hashes_dynamic(&self, trust_fork_after_blocks: usize) -> Vec<types::Hash> {
        let mut vec = vec![];
        if let Some(main) = self.main() {
            let mut hash = main.0;
            for _ in 0..trust_fork_after_blocks {
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
    pub fn insert(&mut self, hash: types::Hash, previous_hash: types::Hash, timestamp: u32) -> Option<bool> {
        if self.hashes.insert(hash, previous_hash).is_some() {
            return None;
        }
        if let Some(index) = self.branches.iter().position(|(hash, _, _)| hash == &previous_hash) {
            // extend branch
            self.branches[index] = (hash, self.branches[index].1 + 1, timestamp);
            Some(false)
        } else {
            // new branch
            self.branches.push((hash, self.height(&previous_hash), timestamp));
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
    pub fn height(&self, previous_hash: &types::Hash) -> usize {
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
            branches: Vec<(String, usize, u32)>,
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
