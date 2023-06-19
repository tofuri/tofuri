use crate::Fork;
use rocksdb::DB;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::VecDeque;
use tofuri_block::Block;
use tofuri_checkpoint::Checkpoint;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stable {
    pub latest_block: Block,
    pub hashes: Vec<[u8; 32]>,
    pub stakers: VecDeque<[u8; 20]>,
    latest_blocks: Vec<Block>,
    map_balance: HashMap<[u8; 20], u128>,
    map_staked: HashMap<[u8; 20], u128>,
}
impl Stable {
    pub fn append_block(&mut self, block_a: &Block, previous_timestamp: u32) {
        crate::append_block(self, block_a, previous_timestamp, false)
    }
    pub fn load(&mut self, db: &DB, hashes: &[[u8; 32]]) {
        crate::load(self, db, hashes)
    }
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            height: self.hashes.len(),
            latest_block: self.latest_block.clone(),
            stakers: self.stakers.clone(),
            latest_blocks: self.latest_blocks.clone(),
            map_balance: self.map_balance.clone(),
            map_staked: self.map_staked.clone(),
        }
    }
    pub fn from_checkpoint(hashes: Vec<[u8; 32]>, checkpoint: Checkpoint) -> Stable {
        Stable {
            latest_block: checkpoint.latest_block,
            hashes,
            stakers: checkpoint.stakers,
            latest_blocks: checkpoint.latest_blocks,
            map_balance: checkpoint.map_balance,
            map_staked: checkpoint.map_staked,
        }
    }
}
impl Fork for Stable {
    fn get_hashes_mut(&mut self) -> &mut Vec<[u8; 32]> {
        &mut self.hashes
    }
    fn get_stakers(&self) -> &VecDeque<[u8; 20]> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<[u8; 20]> {
        &mut self.stakers
    }
    fn get_map_balance(&self) -> &HashMap<[u8; 20], u128> {
        &self.map_balance
    }
    fn get_map_balance_mut(&mut self) -> &mut HashMap<[u8; 20], u128> {
        &mut self.map_balance
    }
    fn get_map_staked(&self) -> &HashMap<[u8; 20], u128> {
        &self.map_staked
    }
    fn get_map_staked_mut(&mut self) -> &mut HashMap<[u8; 20], u128> {
        &mut self.map_staked
    }
    fn get_latest_block(&self) -> &Block {
        &self.latest_block
    }
    fn get_latest_block_mut(&mut self) -> &mut Block {
        &mut self.latest_block
    }
    fn get_latest_blocks(&self) -> &Vec<Block> {
        &self.latest_blocks
    }
    fn get_latest_blocks_mut(&mut self) -> &mut Vec<Block> {
        &mut self.latest_blocks
    }
    fn is_stable() -> bool {
        true
    }
    fn append_block(&mut self, block_a: &Block, previous_timestamp: u32, loading: bool) {
        crate::append_block(self, block_a, previous_timestamp, loading)
    }
}
