use pea_block::Block;
use pea_core::{constants::BLOCK_TIME_MAX, constants::MIN_STAKE, types};
use pea_db as db;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::collections::{HashMap, VecDeque};
pub trait State {
    fn get_hashes_mut(&mut self) -> &mut Vec<types::Hash>;
    fn get_stakers(&self) -> &VecDeque<types::PublicKeyBytes>;
    fn get_stakers_mut(&mut self) -> &mut VecDeque<types::PublicKeyBytes>;
    fn get_balance(&self) -> &HashMap<types::PublicKeyBytes, u128>;
    fn get_balance_mut(&mut self) -> &mut HashMap<types::PublicKeyBytes, u128>;
    fn get_balance_staked(&self) -> &HashMap<types::PublicKeyBytes, u128>;
    fn get_balance_staked_mut(&mut self) -> &mut HashMap<types::PublicKeyBytes, u128>;
    fn balance(&self, public_key: &types::PublicKeyBytes) -> u128;
    fn balance_staked(&self, public_key: &types::PublicKeyBytes) -> u128;
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
    pub stakers: VecDeque<types::PublicKeyBytes>,
    balance: HashMap<types::PublicKeyBytes, u128>,
    balance_staked: HashMap<types::PublicKeyBytes, u128>,
}
#[derive(Default, Debug, Clone)]
pub struct Dynamic {
    pub hashes: Vec<types::Hash>,
    pub stakers: VecDeque<types::PublicKeyBytes>,
    balance: HashMap<types::PublicKeyBytes, u128>,
    balance_staked: HashMap<types::PublicKeyBytes, u128>,
    pub latest_block: Block,
}
impl Trusted {
    pub fn balance(&self, public_key: &types::PublicKeyBytes) -> u128 {
        balance(self, public_key)
    }
    pub fn balance_staked(&self, public_key: &types::PublicKeyBytes) -> u128 {
        balance_staked(self, public_key)
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
    pub fn balance(&self, public_key: &types::PublicKeyBytes) -> u128 {
        balance(self, public_key)
    }
    pub fn balance_staked(&self, public_key: &types::PublicKeyBytes) -> u128 {
        balance_staked(self, public_key)
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
    pub fn staker(&self, timestamp: u32, previous_timestamp: u32) -> Option<&types::PublicKeyBytes> {
        self.stakers.get(staker_index(timestamp, previous_timestamp))
    }
    pub fn current_staker(&self, timestamp: u32) -> Option<&types::PublicKeyBytes> {
        self.staker(timestamp, self.latest_block.timestamp)
    }
    pub fn offline_staker(&self, timestamp: u32) -> Option<&types::PublicKeyBytes> {
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
    fn get_stakers(&self) -> &VecDeque<types::PublicKeyBytes> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<types::PublicKeyBytes> {
        &mut self.stakers
    }
    fn get_balance(&self) -> &HashMap<types::PublicKeyBytes, u128> {
        &self.balance
    }
    fn get_balance_mut(&mut self) -> &mut HashMap<types::PublicKeyBytes, u128> {
        &mut self.balance
    }
    fn get_balance_staked(&self) -> &HashMap<types::PublicKeyBytes, u128> {
        &self.balance_staked
    }
    fn get_balance_staked_mut(&mut self) -> &mut HashMap<types::PublicKeyBytes, u128> {
        &mut self.balance_staked
    }
    fn balance(&self, public_key: &types::PublicKeyBytes) -> u128 {
        balance(self, public_key)
    }
    fn balance_staked(&self, public_key: &types::PublicKeyBytes) -> u128 {
        balance_staked(self, public_key)
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
    fn get_stakers(&self) -> &VecDeque<types::PublicKeyBytes> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<types::PublicKeyBytes> {
        &mut self.stakers
    }
    fn get_balance(&self) -> &HashMap<types::PublicKeyBytes, u128> {
        &self.balance
    }
    fn get_balance_mut(&mut self) -> &mut HashMap<types::PublicKeyBytes, u128> {
        &mut self.balance
    }
    fn get_balance_staked(&self) -> &HashMap<types::PublicKeyBytes, u128> {
        &self.balance_staked
    }
    fn get_balance_staked_mut(&mut self) -> &mut HashMap<types::PublicKeyBytes, u128> {
        &mut self.balance_staked
    }
    fn balance(&self, public_key: &types::PublicKeyBytes) -> u128 {
        balance(self, public_key)
    }
    fn balance_staked(&self, public_key: &types::PublicKeyBytes) -> u128 {
        balance_staked(self, public_key)
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
fn balance<T: State>(state: &T, public_key: &types::PublicKeyBytes) -> u128 {
    match state.get_balance().get(public_key) {
        Some(b) => *b,
        None => 0,
    }
}
fn balance_staked<T: State>(state: &T, public_key: &types::PublicKeyBytes) -> u128 {
    match state.get_balance_staked().get(public_key) {
        Some(b) => *b,
        None => 0,
    }
}
fn update_balances<T: State>(state: &mut T, block: &Block) {
    for transaction in block.transactions.iter() {
        let mut balance_input = state.balance(&transaction.public_key_input);
        let mut balance_output = state.balance(&transaction.public_key_output);
        balance_input -= transaction.amount + transaction.fee;
        balance_output += transaction.amount;
        if balance_input == 0 {
            state.get_balance_mut().remove(&transaction.public_key_input);
        } else {
            state.get_balance_mut().insert(transaction.public_key_input, balance_input);
        }
        state.get_balance_mut().insert(transaction.public_key_output, balance_output);
    }
    for stake in block.stakes.iter() {
        let mut balance = state.balance(&stake.public_key);
        let mut balance_staked = state.balance_staked(&stake.public_key);
        if stake.deposit {
            balance -= stake.amount + stake.fee;
            balance_staked += stake.amount;
        } else {
            balance += stake.amount - stake.fee;
            balance_staked -= stake.amount;
        }
        if balance == 0 {
            state.get_balance_mut().remove(&stake.public_key);
        } else {
            state.get_balance_mut().insert(stake.public_key, balance);
        }
        state.get_balance_staked_mut().insert(stake.public_key, balance_staked);
    }
}
fn update_stakers<T: State>(state: &mut T, block: &Block) {
    if state.get_stakers_mut().len() > 1 {
        state.get_stakers_mut().rotate_left(1);
    }
    for stake in block.stakes.iter() {
        let balance_staked = state.balance_staked(&stake.public_key);
        let any = state.get_stakers_mut().iter().any(|x| x == &stake.public_key);
        if !any && balance_staked >= MIN_STAKE {
            state.get_stakers_mut().push_back(stake.public_key);
        } else if any && balance_staked < MIN_STAKE {
            state.get_balance_staked_mut().remove(&stake.public_key);
            let index = state.get_stakers_mut().iter().position(|x| x == &stake.public_key).unwrap();
            state.get_stakers_mut().remove(index).unwrap();
        }
    }
}
fn update_reward<T: State>(state: &mut T, block: &Block) {
    let balance_staked = state.balance_staked(&block.public_key);
    let mut balance = state.balance(&block.public_key);
    balance += block.reward(balance_staked);
    if let Some(stake) = block.stakes.first() {
        if stake.fee == 0 {
            balance += MIN_STAKE;
        }
    }
    state.get_balance_mut().insert(block.public_key, balance);
}
fn staker_index(timestamp: u32, previous_timestamp: u32) -> usize {
    let diff = timestamp.saturating_sub(previous_timestamp + 1);
    let index = diff / BLOCK_TIME_MAX as u32;
    index as usize
}
fn update_penalty<T: State>(state: &mut T, timestamp: u32, previous_timestamp: u32) {
    for _ in 0..staker_index(timestamp, previous_timestamp) {
        if state.get_stakers_mut().is_empty() {
            break;
        }
        let staker = state.get_stakers().get(0).unwrap().clone();
        state.get_balance_staked_mut().remove(&staker);
        state.get_stakers_mut().remove(0).unwrap();
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
