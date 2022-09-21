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
    pub fn main(&mut self) -> Option<&Branch> {
        self.sort();
        self.branches.first()
    }
    pub fn get(&mut self, hash: &types::Hash) -> Option<&types::Hash> {
        self.hashes.get(hash)
    }
    pub fn insert(&mut self, hash: types::Hash, previous_hash: types::Hash) {
        if self.hashes.insert(hash, previous_hash).is_some() {
            return;
        }
        if let Some(index) = self
            .branches
            .iter()
            .position(|(hash, _)| hash == &previous_hash)
        {
            // extend branch
            self.branches[index] = (hash, self.height(&previous_hash));
        } else {
            // new branch
            self.branches.push((hash, self.height(&previous_hash)));
        }
    }
    fn sort(&mut self) {
        self.branches.sort_by(|a, b| b.1.cmp(&a.1));
    }
    fn height(&self, previous_hash: &types::Hash) -> types::Height {
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
                        // panic!("broken chain")
                        height = 0;
                    }
                    break;
                }
            };
        }
        height
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>) {
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
        let (_, vec) = hashes
            .iter()
            .find(|(&previous_hash, _)| previous_hash == [0; 32])
            .unwrap();
        let previous_hash = [0; 32];
        // let mut closure = || -> bool {
        // if vec.is_empty() {
        // return false;
        // }
        // for hash in vec {
        // self.insert(*hash, previous_hash);
        // closure()
        // }
        // true
        // };
        // while closure() {}
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
        // loop {
        // for hash in vec {
        // self.insert(hash, previous_hash);
        // }
        // }
        // for (previous_hash, vec) in hashes {
        // for hash in vec {
        // self.insert(hash, previous_hash);
        // println!("{}", hex::encode(hash));
        // }
        // }
        println!("{:?}", self);
    }
    // pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>) {
    // let mut hashes: Vec<(types::Hash, types::Hash)> = vec![];
    // for res in db.iterator_cf(db::blocks(db), IteratorMode::Start) {
    // let (hash, bytes) = res.unwrap();
    // let block: BlockMetadataLean = bincode::deserialize(&bytes).unwrap();
    // hashes.push((hash.to_vec().try_into().unwrap(), block.previous_hash));
    // }
    // for (hash, previous_hash) in hashes {
    // self.insert(hash, previous_hash);
    // println!("{}", hex::encode(hash));
    // }
    // println!("{:?}", self);
    // }
    // pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>) {
    // let mut hashes: Hashes = HashMap::new();
    // for res in db.iterator_cf(db::blocks(db), IteratorMode::Start) {
    // let (hash, bytes) = res.unwrap();
    // let block: BlockMetadataLean = bincode::deserialize(&bytes).unwrap();
    // hashes.insert(hash.to_vec().try_into().unwrap(), block.previous_hash);
    // // println!("{:?}", hex::encode(document.unwrap().0));
    // }
    // let mut hashes2 = HashMap::new();
    // loop {
    // println!("{} - {}", hashes.len(), hashes2.len());
    // if hashes.len() == hashes2.len() {
    // hashes2.clear();
    // break
    // }
    // let mut hash = hashes.iter().last().unwrap().0;
    // let mut branch = vec![];
    // loop {
    // match hashes.get(hash) {
    // Some(previous_hash) => {
    // branch.push(hash);
    // hashes2.insert(hash, previous_hash);
    // hash = &previous_hash;
    // }
    // None => {
    // if hash != &[0; 32] {
    // panic!("broken chain")
    // }
    // break;
    // }
    // };
    // }
    // println!("{:?}", branch);
    // println!("{}", hex::encode(hash));
    // for (index, hash) in branch.iter().enumerate() {
    // if index == 0 {
    // self.insert(**hash, [0; 32])
    // } else {
    // self.insert(**hash, *branch[index - 1])
    // }
    // }
    // }
    // }
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
