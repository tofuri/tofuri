use pea_block::Block;
use pea_core::constants::{COIN, STAKE};
use pea_core::util;
use pea_core::{constants::BLOCK_TIME_MAX, types};
use pea_db as db;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::collections::{HashMap, VecDeque};
pub trait State {
    fn get_hashes_mut(&mut self) -> &mut Vec<types::Hash>;
    fn get_stakers(&self) -> &VecDeque<types::AddressBytes>;
    fn get_stakers_mut(&mut self) -> &mut VecDeque<types::AddressBytes>;
    fn get_balance(&self) -> &HashMap<types::AddressBytes, u128>;
    fn get_balance_mut(&mut self) -> &mut HashMap<types::AddressBytes, u128>;
    fn get_balance_staked(&self) -> &HashMap<types::AddressBytes, u128>;
    fn get_balance_staked_mut(&mut self) -> &mut HashMap<types::AddressBytes, u128>;
    fn balance(&self, address: &types::AddressBytes) -> u128;
    fn balance_staked(&self, address: &types::AddressBytes) -> u128;
    fn update_balances(&mut self, block: &Block);
    fn update_stakers(&mut self, block: &Block);
    fn update_reward(&mut self, block: &Block);
    fn update_penalty(&mut self, timestamp: u32, previous_timestamp: u32);
    fn update(&mut self, block: &Block, previous_timestamp: u32);
    fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[types::Hash]);
}
#[derive(Default, Debug, Clone)]
pub struct Trusted {
    pub hashes: Vec<types::Hash>,
    pub stakers: VecDeque<types::AddressBytes>,
    balance: HashMap<types::AddressBytes, u128>,
    balance_staked: HashMap<types::AddressBytes, u128>,
}
#[derive(Default, Debug, Clone)]
pub struct Dynamic {
    pub hashes: Vec<types::Hash>,
    pub stakers: VecDeque<types::AddressBytes>,
    balance: HashMap<types::AddressBytes, u128>,
    balance_staked: HashMap<types::AddressBytes, u128>,
    pub latest_block: Block,
}
impl Trusted {
    pub fn balance(&self, address: &types::AddressBytes) -> u128 {
        balance(self, address)
    }
    pub fn balance_staked(&self, address: &types::AddressBytes) -> u128 {
        balance_staked(self, address)
    }
    pub fn update_balances(&mut self, block: &Block) {
        update_balances(self, block)
    }
    pub fn update_stakers(&mut self, block: &Block) {
        update_stakers(self, block)
    }
    pub fn update_reward(&mut self, block: &Block) {
        update_reward(self, block)
    }
    pub fn update_penalty(&mut self, timestamp: u32, previous_timestamp: u32) {
        update_penalty(self, timestamp, previous_timestamp)
    }
    pub fn update(&mut self, block: &Block, previous_timestamp: u32) {
        update(self, block, previous_timestamp)
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[types::Hash]) {
        load(self, db, hashes)
    }
}
impl Dynamic {
    pub fn balance(&self, address: &types::AddressBytes) -> u128 {
        balance(self, address)
    }
    pub fn balance_staked(&self, address: &types::AddressBytes) -> u128 {
        balance_staked(self, address)
    }
    pub fn update_balances(&mut self, block: &Block) {
        update_balances(self, block)
    }
    pub fn update_stakers(&mut self, block: &Block) {
        update_stakers(self, block)
    }
    pub fn update_reward(&mut self, block: &Block) {
        update_reward(self, block)
    }
    pub fn update_penalty(&mut self, timestamp: u32, previous_timestamp: u32) {
        update_penalty(self, timestamp, previous_timestamp)
    }
    pub fn update(&mut self, block: &Block, previous_timestamp: u32) {
        update(self, block, previous_timestamp)
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[types::Hash]) {
        load(self, db, hashes)
    }
    pub fn from(db: &DBWithThreadMode<SingleThreaded>, hashes: &[types::Hash], trusted: &Trusted) -> Dynamic {
        let mut dynamic = Self {
            hashes: vec![],
            stakers: trusted.stakers.clone(),
            balance: trusted.balance.clone(),
            balance_staked: trusted.balance_staked.clone(),
            latest_block: Block::default(),
        };
        dynamic.load(db, hashes);
        if let Some(hash) = hashes.last() {
            dynamic.latest_block = db::block::get(db, hash).unwrap()
        };
        dynamic
    }
    pub fn staker(&self, timestamp: u32, previous_timestamp: u32) -> Option<&types::AddressBytes> {
        self.stakers.get(staker_index(timestamp, previous_timestamp))
    }
    pub fn current_staker(&self, timestamp: u32) -> Option<&types::AddressBytes> {
        self.staker(timestamp, self.latest_block.timestamp)
    }
    pub fn offline_staker(&self, timestamp: u32) -> Option<&types::AddressBytes> {
        let index = staker_index(timestamp, self.latest_block.timestamp);
        if index == 0 {
            return None;
        }
        self.stakers.get(index - 1)
    }
}
impl State for Trusted {
    fn get_hashes_mut(&mut self) -> &mut Vec<types::Hash> {
        &mut self.hashes
    }
    fn get_stakers(&self) -> &VecDeque<types::AddressBytes> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<types::AddressBytes> {
        &mut self.stakers
    }
    fn get_balance(&self) -> &HashMap<types::AddressBytes, u128> {
        &self.balance
    }
    fn get_balance_mut(&mut self) -> &mut HashMap<types::AddressBytes, u128> {
        &mut self.balance
    }
    fn get_balance_staked(&self) -> &HashMap<types::AddressBytes, u128> {
        &self.balance_staked
    }
    fn get_balance_staked_mut(&mut self) -> &mut HashMap<types::AddressBytes, u128> {
        &mut self.balance_staked
    }
    fn balance(&self, address: &types::AddressBytes) -> u128 {
        balance(self, address)
    }
    fn balance_staked(&self, address: &types::AddressBytes) -> u128 {
        balance_staked(self, address)
    }
    fn update_balances(&mut self, block: &Block) {
        update_balances(self, block)
    }
    fn update_stakers(&mut self, block: &Block) {
        update_stakers(self, block)
    }
    fn update_reward(&mut self, block: &Block) {
        update_reward(self, block)
    }
    fn update_penalty(&mut self, timestamp: u32, previous_timestamp: u32) {
        update_penalty(self, timestamp, previous_timestamp)
    }
    fn update(&mut self, block: &Block, previous_timestamp: u32) {
        update(self, block, previous_timestamp)
    }
    fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[types::Hash]) {
        load(self, db, hashes)
    }
}
impl State for Dynamic {
    fn get_hashes_mut(&mut self) -> &mut Vec<types::Hash> {
        &mut self.hashes
    }
    fn get_stakers(&self) -> &VecDeque<types::AddressBytes> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<types::AddressBytes> {
        &mut self.stakers
    }
    fn get_balance(&self) -> &HashMap<types::AddressBytes, u128> {
        &self.balance
    }
    fn get_balance_mut(&mut self) -> &mut HashMap<types::AddressBytes, u128> {
        &mut self.balance
    }
    fn get_balance_staked(&self) -> &HashMap<types::AddressBytes, u128> {
        &self.balance_staked
    }
    fn get_balance_staked_mut(&mut self) -> &mut HashMap<types::AddressBytes, u128> {
        &mut self.balance_staked
    }
    fn balance(&self, address: &types::AddressBytes) -> u128 {
        balance(self, address)
    }
    fn balance_staked(&self, address: &types::AddressBytes) -> u128 {
        balance_staked(self, address)
    }
    fn update_balances(&mut self, block: &Block) {
        update_balances(self, block)
    }
    fn update_stakers(&mut self, block: &Block) {
        update_stakers(self, block)
    }
    fn update_reward(&mut self, block: &Block) {
        update_reward(self, block)
    }
    fn update_penalty(&mut self, timestamp: u32, previous_timestamp: u32) {
        update_penalty(self, timestamp, previous_timestamp)
    }
    fn update(&mut self, block: &Block, previous_timestamp: u32) {
        update(self, block, previous_timestamp)
    }
    fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[types::Hash]) {
        load(self, db, hashes)
    }
}
fn balance<T: State>(state: &T, address: &types::AddressBytes) -> u128 {
    match state.get_balance().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn balance_staked<T: State>(state: &T, address: &types::AddressBytes) -> u128 {
    match state.get_balance_staked().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn update_balances<T: State>(state: &mut T, block: &Block) {
    for transaction in block.transactions.iter() {
        let input_address = util::address(&transaction.input_public_key);
        let mut balance_input = state.balance(&input_address);
        let mut balance_output = state.balance(&transaction.output_address);
        balance_input -= transaction.amount + transaction.fee;
        balance_output += transaction.amount;
        if balance_input == 0 {
            state.get_balance_mut().remove(&input_address);
        } else {
            state.get_balance_mut().insert(input_address, balance_input);
        }
        state.get_balance_mut().insert(transaction.output_address, balance_output);
    }
    for stake in block.stakes.iter() {
        let address = util::address(&stake.public_key);
        let mut balance = state.balance(&address);
        let mut balance_staked = state.balance_staked(&address);
        if stake.deposit {
            balance -= STAKE + stake.fee;
            balance_staked += STAKE;
        } else {
            balance += STAKE - stake.fee;
            balance_staked -= STAKE;
        }
        if balance == 0 {
            state.get_balance_mut().remove(&address);
        } else {
            state.get_balance_mut().insert(address, balance);
        }
        if balance_staked == 0 {
            state.get_balance_staked_mut().remove(&address);
        } else {
            state.get_balance_staked_mut().insert(address, balance_staked);
        }
    }
}
fn update_staker<T: State>(state: &mut T, address: types::AddressBytes) {
    let balance_staked = state.balance_staked(&address);
    let any = state.get_stakers().iter().any(|x| x == &address);
    if !any && balance_staked >= COIN {
        state.get_stakers_mut().push_back(address);
    } else if any && balance_staked < COIN {
        state.get_balance_staked_mut().remove(&address);
        let index = state.get_stakers().iter().position(|x| x == &address).unwrap();
        state.get_stakers_mut().remove(index).unwrap();
    }
}
fn update_stakers<T: State>(state: &mut T, block: &Block) {
    if state.get_stakers().len() > 1 {
        state.get_stakers_mut().rotate_left(1);
    }
    for stake in block.stakes.iter() {
        update_staker(state, util::address(&stake.public_key));
    }
}
fn update_reward<T: State>(state: &mut T, block: &Block) {
    let address = util::address(&block.public_key);
    let mut balance = state.balance(&address);
    balance += block.reward();
    if let Some(stake) = block.stakes.first() {
        if stake.fee == 0 {
            balance += STAKE;
        }
    }
    state.get_balance_mut().insert(address, balance);
}
fn staker_index(timestamp: u32, previous_timestamp: u32) -> usize {
    let diff = timestamp.saturating_sub(previous_timestamp + 1);
    let index = diff / BLOCK_TIME_MAX as u32;
    index as usize
}
fn update_penalty<T: State>(state: &mut T, timestamp: u32, previous_timestamp: u32) {
    for i in 0..staker_index(timestamp, previous_timestamp) {
        if state.get_stakers_mut().is_empty() {
            break;
        }
        let address = state.get_stakers().get(0).unwrap().clone();
        let mut balance_staked = state.balance(&address);
        balance_staked = balance_staked.saturating_sub(COIN * i as u128);
        state.get_balance_staked_mut().insert(address, balance_staked);
        update_staker(state, address);
    }
}
pub fn update<T: State>(state: &mut T, block: &Block, previous_timestamp: u32) {
    state.get_hashes_mut().push(block.hash());
    state.update_penalty(block.timestamp, previous_timestamp);
    state.update_reward(block);
    state.update_balances(block);
    state.update_stakers(block);
}
pub fn load<T: State>(state: &mut T, db: &DBWithThreadMode<SingleThreaded>, hashes: &[types::Hash]) {
    let mut previous_timestamp = match hashes.first() {
        Some(hash) => db::block::get(db, hash).unwrap().timestamp,
        None => 0,
    };
    for hash in hashes.iter() {
        let block = db::block::get(db, hash).unwrap();
        state.update(&block, previous_timestamp);
        previous_timestamp = block.timestamp;
    }
}
