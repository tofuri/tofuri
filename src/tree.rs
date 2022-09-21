use std::collections::HashMap;
use crate::types;
pub struct Tree {
    branches: Vec<(types::Hash, types::Amount)>, // <(leaf, stake sum)>
    hashes: HashMap<types::Hash, types::Hash>, // <hash, previous hash>
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
    pub fn main(&mut self) -> Option<types::Hash> {
        self.sort();
        match self.branches.first() {
            Some(branch) => Some(branch.0),
            None => None,
        }
    }
    // pub fn main(&mut self) -> Option<&([u8; 32], u128)> {
    // self.sort();
    // self.branches.first()
    // }
}
