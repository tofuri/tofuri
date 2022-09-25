use crate::{block::Block, state::State, types};
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
    pub fn append(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        block: &Block,
        height: types::Height,
    ) {
        self.current.append(block.clone(), height);
        let hashes = self.current.get_hashes();
        let len = hashes.len();
        if len >= 100 {
            let height = len - 100;
            let hash = hashes[height];
            let block = Block::get(db, &hash).unwrap();
            self.previous.append(block, height);
        }
    }
    pub fn reload(&mut self, db: &DBWithThreadMode<SingleThreaded>, mut hashes: Vec<types::Hash>) {
        self.current.reload(db, hashes.clone(), true);
        let len = hashes.len();
        let start = if len < 100 { 0 } else { len - 100 };
        hashes.drain(start..len);
        self.previous.reload(db, hashes, false);
    }
}
