use std::cmp::Ordering;
use std::collections::HashMap;
use tofuri_core::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Branch {
    pub hash: Hash,
    pub height: usize,
    pub timestamp: u32,
}
impl Branch {
    fn new(hash: Hash, height: usize, timestamp: u32) -> Branch {
        Branch { hash, height, timestamp }
    }
}
#[derive(Default, Debug, Clone)]
pub struct Tree {
    branches: Vec<Branch>,
    hashes: HashMap<Hash, Hash>,
}
impl Tree {
    pub fn main(&self) -> Option<&Branch> {
        self.branches.first()
    }
    pub fn size(&self) -> usize {
        self.hashes.len()
    }
    pub fn stable_and_unstable_hashes(&self, trust_fork_after_blocks: usize) -> (Vec<Hash>, Vec<Hash>) {
        let mut stable_hashes = vec![];
        if let Some(main) = self.main() {
            let mut hash = main.hash;
            loop {
                stable_hashes.push(hash);
                match self.get(&hash) {
                    Some(previous_hash) => hash = *previous_hash,
                    None => break,
                };
            }
        }
        if let Some(hash) = stable_hashes.last() {
            if hash != &GENESIS_BLOCK_PREVIOUS_HASH {
                panic!("broken chain")
            }
            stable_hashes.pop();
        }
        stable_hashes.reverse();
        let len = stable_hashes.len();
        let start = if len < trust_fork_after_blocks { 0 } else { len - trust_fork_after_blocks };
        let unstable_hashes = stable_hashes.drain(start..len).collect();
        (stable_hashes, unstable_hashes)
    }
    pub fn unstable_hashes(&self, trust_fork_after_blocks: usize) -> Vec<Hash> {
        let mut vec = vec![];
        if let Some(main) = self.main() {
            let mut hash = main.hash;
            for _ in 0..trust_fork_after_blocks {
                vec.push(hash);
                match self.get(&hash) {
                    Some(previous_hash) => hash = *previous_hash,
                    None => break,
                };
            }
        }
        if let Some(hash) = vec.last() {
            if hash == &GENESIS_BLOCK_PREVIOUS_HASH {
                vec.pop();
            }
        }
        vec.reverse();
        vec
    }
    pub fn get(&self, hash: &Hash) -> Option<&Hash> {
        self.hashes.get(hash)
    }
    pub fn insert(&mut self, hash: Hash, previous_hash: Hash, timestamp: u32) -> Option<bool> {
        if self.hashes.insert(hash, previous_hash).is_some() {
            return None;
        }
        if let Some(index) = self.branches.iter().position(|a| a.hash == previous_hash) {
            // extend branch
            self.branches[index] = Branch::new(hash, self.branches[index].height + 1, timestamp);
            Some(false)
        } else {
            // new branch
            self.branches.push(Branch::new(hash, self.height(&previous_hash), timestamp));
            Some(true)
        }
    }
    pub fn sort_branches(&mut self) {
        self.branches.sort_by(|a, b| match b.height.cmp(&a.height) {
            Ordering::Equal => a.timestamp.cmp(&b.timestamp),
            x => x,
        });
    }
    pub fn clear(&mut self) {
        self.branches.clear();
        self.hashes.clear();
    }
    fn height(&self, previous_hash: &Hash) -> usize {
        let mut hash = previous_hash;
        let mut height = 1;
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
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        let mut tree = Tree::default();
        tree.insert([0x11; 32], [0x00; 32], 1);
        tree.insert([0x22; 32], [0x11; 32], 1);
        tree.insert([0x33; 32], [0x22; 32], 1);
        assert_eq!(tree.size(), 3);
        tree.insert([0x44; 32], [0x33; 32], 1);
        tree.insert([0x55; 32], [0x22; 32], 1);
        tree.insert([0x66; 32], [0x00; 32], 1);
        tree.insert([0x77; 32], [0x55; 32], 0);
        assert_eq!(tree.main(), Some(&Branch::new([0x44; 32], 3, 1)));
        tree.sort_branches();
        assert_eq!(tree.main(), Some(&Branch::new([0x77; 32], 3, 0)));
        assert_eq!(tree.size(), 7);
    }
}
