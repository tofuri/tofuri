use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::VecDeque;
use tofuri_block::BlockA;
use tofuri_core::*;
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub height: usize,
    pub latest_block: BlockA,
    pub stakers: VecDeque<AddressBytes>,
    pub latest_blocks: Vec<BlockA>,
    pub map_balance: HashMap<AddressBytes, u128>,
    pub map_staked: HashMap<AddressBytes, u128>,
}
