use std::cmp::Ordering;
use std::collections::HashMap;
type Hash = [u8; 32];
type Branch = (Hash, usize, u32);
#[derive(Debug)]
pub struct Tree {
    branches: Vec<Branch>,
    hashes: HashMap<Hash, Hash>,
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
    pub fn hashes(&self, trust_fork_after_blocks: usize) -> (Vec<Hash>, Vec<Hash>) {
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
    pub fn hashes_dynamic(&self, trust_fork_after_blocks: usize) -> Vec<Hash> {
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
    pub fn get(&self, hash: &Hash) -> Option<&Hash> {
        self.hashes.get(hash)
    }
    pub fn insert(&mut self, hash: Hash, previous_hash: Hash, timestamp: u32) -> Option<bool> {
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
            Ordering::Equal => a.2.cmp(&b.2),
            x => x,
        });
    }
    pub fn height(&self, previous_hash: &Hash) -> usize {
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
impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        let mut tree = Tree::new();
        tree.insert([0x11; 32], [0x00; 32], 1);
        tree.insert([0x22; 32], [0x11; 32], 1);
        tree.insert([0x33; 32], [0x22; 32], 1);
        assert_eq!(tree.size(), 3);
        tree.insert([0x44; 32], [0x33; 32], 1);
        tree.insert([0x55; 32], [0x22; 32], 1);
        tree.insert([0x66; 32], [0x00; 32], 1);
        tree.insert([0x77; 32], [0x55; 32], 0);
        assert_eq!(tree.main(), Some(&([0x44; 32], 3, 1)));
        tree.sort_branches();
        assert_eq!(tree.main(), Some(&([0x77; 32], 3, 0)));
        assert_eq!(tree.size(), 7);
    }
}
