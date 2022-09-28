use crate::{block::BlockMetadataLean, db, types};
use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
use std::collections::HashMap;
use std::fmt;
type Branch = (types::Hash, types::Height);
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
    pub fn get_vec(&self) -> Vec<types::Hash> {
        let mut vec = vec![];
        if let Some(main) = self.main() {
            let mut hash = main.0;
            loop {
                vec.push(hash);
                match self.get(&hash) {
                    Some(previous_hash) => hash = *previous_hash,
                    None => break,
                };
            }
        }
        if let Some(hash) = vec.last() {
            if hash != &[0; 32] {
                panic!("broken chain")
            }
            vec.pop();
        }
        vec.reverse();
        vec
    }
    pub fn get_fork_vec(&self, hashes: &[types::Hash], mut hash: types::Hash) -> Vec<types::Hash> {
        let mut vec = vec![];
        loop {
            vec.push(hash);
            if hashes.contains(&hash) {
                break;
            }
            match self.get(&hash) {
                Some(previous_hash) => hash = *previous_hash,
                None => break,
            };
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
    pub fn insert(&mut self, hash: types::Hash, previous_hash: types::Hash) -> Option<bool> {
        if self.hashes.insert(hash, previous_hash).is_some() {
            return None;
        }
        if let Some(index) = self
            .branches
            .iter()
            .position(|(hash, _)| hash == &previous_hash)
        {
            // extend branch
            self.branches[index] = (hash, self.height(&previous_hash));
            Some(false)
        } else {
            // new branch
            self.branches.push((hash, self.height(&previous_hash)));
            Some(true)
        }
    }
    pub fn sort_branches(&mut self) {
        self.branches.sort_by(|a, b| b.1.cmp(&a.1));
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
        let mut hashes: HashMap<types::Hash, Vec<types::Hash>> = HashMap::new();
        for res in db.iterator_cf(db::blocks(db), IteratorMode::Start) {
            let (hash, bytes) = res.unwrap();
            let hash = hash.to_vec().try_into().unwrap();
            let block: BlockMetadataLean = bincode::deserialize(&bytes).unwrap();
            match hashes.get(&block.previous_hash) {
                Some(vec) => {
                    let mut vec = vec.clone();
                    vec.push(hash);
                    hashes.insert(block.previous_hash, vec);
                }
                None => {
                    hashes.insert(block.previous_hash, vec![hash]);
                }
            };
        }
        if hashes.is_empty() {
            return;
        }
        let previous_hash = [0; 32];
        let (_, vec) = hashes.iter().find(|(&x, _)| x == previous_hash).unwrap();
        fn recurse(
            tree: &mut Tree,
            hashes: &HashMap<types::Hash, Vec<types::Hash>>,
            previous_hash: types::Hash,
            vec: &Vec<types::Hash>,
        ) {
            for hash in vec {
                tree.insert(*hash, previous_hash);
                if let Some(vec) = hashes.get(hash) {
                    recurse(tree, hashes, *hash, vec);
                };
            }
        }
        recurse(self, &hashes, previous_hash, vec);
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
            branches: Vec<(String, types::Height)>,
            hashes: HashMap<String, String>,
        }
        write!(
            f,
            "{:?}",
            Tree {
                branches: self
                    .branches
                    .iter()
                    .map(|(hash, height)| (hex::encode(hash), *height))
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
