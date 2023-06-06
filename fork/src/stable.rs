use crate::Fork;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::VecDeque;
use tofuri_block::BlockA;
use tofuri_checkpoint::Checkpoint;
use tofuri_core::*;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stable {
    pub latest_block: BlockA,
    pub hashes: Vec<Hash>,
    pub stakers: VecDeque<AddressBytes>,
    latest_blocks: Vec<BlockA>,
    map_balance: HashMap<AddressBytes, u128>,
    map_staked: HashMap<AddressBytes, u128>,
}
impl Stable {
    pub fn append_block(&mut self, block_a: &BlockA, previous_timestamp: u32) {
        crate::append_block(self, block_a, previous_timestamp, false)
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
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
    fn get_hashes_mut(&mut self) -> &mut Vec<Hash> {
        &mut self.hashes
    }
    fn get_stakers(&self) -> &VecDeque<AddressBytes> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<AddressBytes> {
        &mut self.stakers
    }
    fn get_map_balance(&self) -> &HashMap<AddressBytes, u128> {
        &self.map_balance
    }
    fn get_map_balance_mut(&mut self) -> &mut HashMap<AddressBytes, u128> {
        &mut self.map_balance
    }
    fn get_map_staked(&self) -> &HashMap<AddressBytes, u128> {
        &self.map_staked
    }
    fn get_map_staked_mut(&mut self) -> &mut HashMap<AddressBytes, u128> {
        &mut self.map_staked
    }
    fn get_latest_block(&self) -> &BlockA {
        &self.latest_block
    }
    fn get_latest_block_mut(&mut self) -> &mut BlockA {
        &mut self.latest_block
    }
    fn get_latest_blocks(&self) -> &Vec<BlockA> {
        &self.latest_blocks
    }
    fn get_latest_blocks_mut(&mut self) -> &mut Vec<BlockA> {
        &mut self.latest_blocks
    }
    fn is_stable() -> bool {
        true
    }
    fn append_block(&mut self, block_a: &BlockA, previous_timestamp: u32, loading: bool) {
        crate::append_block(self, block_a, previous_timestamp, loading)
    }
}
