use crate::{
    block::Block, blockchain::Blockchain, constants::TRUST_FORK_AFTER_BLOCKS, state::State, types,
};
use rocksdb::{DBWithThreadMode, SingleThreaded};
#[derive(Debug)]
pub struct States {
    current: State,
    previous: State,
}
impl States {
    pub fn new() -> States {
        States {
            current: State::new(),
            previous: State::new(),
        }
    }
    pub fn get_current(&self) -> &State {
        &self.current
    }
    pub fn get_current_mut(&mut self) -> &mut State {
        &mut self.current
    }
    pub fn get_previous(&self) -> &State {
        &self.previous
    }
    pub fn get_previous_mut(&mut self) -> &mut State {
        &mut self.previous
    }
    pub fn get_fork_state(&self, blockchain: &Blockchain, previous_hash: &types::Hash) -> State {
        if previous_hash == &[0; 32] {
            return State::new();
        }
        let vec = blockchain
            .get_tree()
            .get_fork_vec(self.current.get_hashes(), *previous_hash);
        let mut fork_state = self.previous.clone();
        if let Some(hash) = vec.first() {
            if self.previous.get_position(hash).unwrap()
                < blockchain.get_height() - TRUST_FORK_AFTER_BLOCKS
            {
                return State::new();
            }
        }
        for hash in vec.iter() {
            let block = Block::get(blockchain.get_db(), hash).unwrap();
            fork_state.append(block);
        }
        fork_state
    }
    pub fn append(&mut self, db: &DBWithThreadMode<SingleThreaded>, block: &Block) {
        self.current.append(block.clone());
        let hashes = self.current.get_hashes();
        let len = hashes.len();
        if len >= TRUST_FORK_AFTER_BLOCKS {
            let block = Block::get(db, &hashes[len - TRUST_FORK_AFTER_BLOCKS]).unwrap();
            self.previous.append(block);
        }
    }
    pub fn reload(&mut self, db: &DBWithThreadMode<SingleThreaded>, mut hashes: Vec<types::Hash>) {
        self.current.reload(db, hashes.clone(), true);
        let len = hashes.len();
        let start = if len < TRUST_FORK_AFTER_BLOCKS {
            0
        } else {
            len - TRUST_FORK_AFTER_BLOCKS
        };
        hashes.drain(start..len);
        self.previous.reload(db, hashes, false);
    }
}
