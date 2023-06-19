use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::VecDeque;
use tofuri_block::BlockB;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub height: usize,
    pub latest_block: BlockB,
    pub stakers: VecDeque<[u8; 20]>,
    pub latest_blocks: Vec<BlockB>,
    pub map_balance: HashMap<[u8; 20], u128>,
    pub map_staked: HashMap<[u8; 20], u128>,
}
