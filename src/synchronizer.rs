use crate::{block::Block, constants::SYNC_HISTORY_LENGTH, types};
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Synchronizer {
    pub index: usize,
    pub new: usize,
    pub bps: usize, // new blocks per second
    pub history: [usize; SYNC_HISTORY_LENGTH],
}
impl Default for Synchronizer {
    fn default() -> Self {
        Self::new()
    }
}
impl Synchronizer {
    pub fn new() -> Synchronizer {
        Synchronizer {
            index: 0,
            new: 0,
            bps: 0,
            history: [0; SYNC_HISTORY_LENGTH],
        }
    }
    pub fn heartbeat_handle(&mut self) {
        self.history.rotate_right(1);
        self.history[0] = self.new;
        self.new = 0;
        self.bps = 0;
        for x in self.history {
            self.bps += x;
        }
        self.bps /= SYNC_HISTORY_LENGTH;
    }
    pub fn get_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        hashes: &types::Hashes,
    ) -> Block {
        if self.index >= hashes.len() {
            self.index = 0;
        }
        Block::get(db, &hashes[self.index]).unwrap()
    }
}
