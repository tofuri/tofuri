use crate::types;
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
    fn sort(&mut self) {
        self.branches.sort_by(|a, b| b.1.cmp(&a.1));
    }
    pub fn main(&mut self) -> Option<&Branch> {
        self.sort();
        self.branches.first()
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
            self.branches[index] = (hash, self.height(previous_hash));
        } else {
            // new branch
            self.branches.push((hash, self.height(previous_hash)));
        }
    }
    fn height(&self, previous_hash: types::Hash) -> types::Height {
        let mut hash = previous_hash;
        let mut height = 0;
        loop {
            match self.hashes.get(&hash) {
                Some(previous_hash) => {
                    hash = *previous_hash;
                    height += 1;
                }
                None => {
                    if hash != [0; 32] {
                        panic!("broken chain")
                    }
                    break;
                }
            };
        }
        height
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
