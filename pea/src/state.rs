use crate::util;
use colored::Colorize;
use log::warn;
use pea_address::address;
use pea_block::BlockA;
use pea_core::*;
use pea_db as db;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::collections::{HashMap, VecDeque};
pub trait State {
    fn get_hashes_mut(&mut self) -> &mut Vec<Hash>;
    fn get_stakers(&self) -> &VecDeque<AddressBytes>;
    fn get_stakers_mut(&mut self) -> &mut VecDeque<AddressBytes>;
    fn get_balance(&self) -> &HashMap<AddressBytes, u128>;
    fn get_balance_mut(&mut self) -> &mut HashMap<AddressBytes, u128>;
    fn get_staked(&self) -> &HashMap<AddressBytes, u128>;
    fn get_staked_mut(&mut self) -> &mut HashMap<AddressBytes, u128>;
    fn get_latest_block(&self) -> &BlockA;
    fn get_latest_block_mut(&mut self) -> &mut BlockA;
    fn balance(&self, address: &AddressBytes) -> u128;
    fn staked(&self, address: &AddressBytes) -> u128;
    fn append_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, block: &BlockA, previous_timestamp: u32, loading: bool);
    fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]);
    fn staker_n(&self, n: isize) -> Option<AddressBytes>;
}
#[derive(Default, Debug, Clone)]
pub struct Trusted {
    pub hashes: Vec<Hash>,
    pub stakers: VecDeque<AddressBytes>,
    balance: HashMap<AddressBytes, u128>,
    staked: HashMap<AddressBytes, u128>,
    pub latest_block: BlockA,
}
#[derive(Default, Debug, Clone)]
pub struct Dynamic {
    pub hashes: Vec<Hash>,
    pub stakers: VecDeque<AddressBytes>,
    balance: HashMap<AddressBytes, u128>,
    staked: HashMap<AddressBytes, u128>,
    pub latest_block: BlockA,
}
impl Trusted {
    pub fn append_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, block: &BlockA, previous_timestamp: u32) {
        append_block(self, db, block, previous_timestamp, false)
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
        load(self, db, hashes)
    }
}
impl Dynamic {
    pub fn balance(&self, address: &AddressBytes) -> u128 {
        balance(self, address)
    }
    pub fn staked(&self, address: &AddressBytes) -> u128 {
        staked(self, address)
    }
    pub fn from(db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash], trusted: &Trusted) -> Dynamic {
        let mut dynamic = Self {
            hashes: vec![],
            stakers: trusted.stakers.clone(),
            balance: trusted.balance.clone(),
            staked: trusted.staked.clone(),
            latest_block: BlockA::default(),
        };
        dynamic.load(db, hashes);
        dynamic
    }
    pub fn staker_n(&self, n: isize) -> Option<AddressBytes> {
        staker_n(self, n)
    }
    pub fn staker(&self, timestamp: u32) -> Option<AddressBytes> {
        staker_n(self, offline(timestamp, self.get_latest_block().timestamp) as isize)
    }
    pub fn staker_offline(&self, timestamp: u32) -> Option<AddressBytes> {
        staker_n(self, offline(timestamp, self.get_latest_block().timestamp) as isize - 1)
    }
}
impl State for Trusted {
    fn get_hashes_mut(&mut self) -> &mut Vec<Hash> {
        &mut self.hashes
    }
    fn get_stakers(&self) -> &VecDeque<AddressBytes> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<AddressBytes> {
        &mut self.stakers
    }
    fn get_balance(&self) -> &HashMap<AddressBytes, u128> {
        &self.balance
    }
    fn get_balance_mut(&mut self) -> &mut HashMap<AddressBytes, u128> {
        &mut self.balance
    }
    fn get_staked(&self) -> &HashMap<AddressBytes, u128> {
        &self.staked
    }
    fn get_staked_mut(&mut self) -> &mut HashMap<AddressBytes, u128> {
        &mut self.staked
    }
    fn get_latest_block(&self) -> &BlockA {
        &self.latest_block
    }
    fn get_latest_block_mut(&mut self) -> &mut BlockA {
        &mut self.latest_block
    }
    fn balance(&self, address: &AddressBytes) -> u128 {
        balance(self, address)
    }
    fn staked(&self, address: &AddressBytes) -> u128 {
        staked(self, address)
    }
    fn append_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, block: &BlockA, previous_timestamp: u32, loading: bool) {
        append_block(self, db, block, previous_timestamp, loading)
    }
    fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
        load(self, db, hashes)
    }
    fn staker_n(&self, n: isize) -> Option<AddressBytes> {
        staker_n(self, n)
    }
}
impl State for Dynamic {
    fn get_hashes_mut(&mut self) -> &mut Vec<Hash> {
        &mut self.hashes
    }
    fn get_stakers(&self) -> &VecDeque<AddressBytes> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<AddressBytes> {
        &mut self.stakers
    }
    fn get_balance(&self) -> &HashMap<AddressBytes, u128> {
        &self.balance
    }
    fn get_balance_mut(&mut self) -> &mut HashMap<AddressBytes, u128> {
        &mut self.balance
    }
    fn get_staked(&self) -> &HashMap<AddressBytes, u128> {
        &self.staked
    }
    fn get_staked_mut(&mut self) -> &mut HashMap<AddressBytes, u128> {
        &mut self.staked
    }
    fn get_latest_block(&self) -> &BlockA {
        &self.latest_block
    }
    fn get_latest_block_mut(&mut self) -> &mut BlockA {
        &mut self.latest_block
    }
    fn balance(&self, address: &AddressBytes) -> u128 {
        balance(self, address)
    }
    fn staked(&self, address: &AddressBytes) -> u128 {
        staked(self, address)
    }
    fn append_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, block: &BlockA, previous_timestamp: u32, loading: bool) {
        append_block(self, db, block, previous_timestamp, loading)
    }
    fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
        load(self, db, hashes)
    }
    fn staker_n(&self, n: isize) -> Option<AddressBytes> {
        staker_n(self, n)
    }
}
fn balance<T: State>(state: &T, address: &AddressBytes) -> u128 {
    match state.get_balance().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn staked<T: State>(state: &T, address: &AddressBytes) -> u128 {
    match state.get_staked().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn balance_insert<T: State>(state: &mut T, address: AddressBytes, balance: u128) {
    match balance {
        0 => state.get_balance_mut().remove(&address),
        x => state.get_balance_mut().insert(address, x),
    };
}
fn staked_insert<T: State>(state: &mut T, address: AddressBytes, staked: u128) {
    match staked {
        0 => state.get_staked_mut().remove(&address),
        x => state.get_staked_mut().insert(address, x),
    };
}
fn update_staker<T: State>(state: &mut T, address: AddressBytes) {
    let staked = state.staked(&address);
    let index = state.get_stakers().iter().position(|x| x == &address);
    if index.is_none() && staked >= COIN {
        state.get_stakers_mut().push_back(address);
    } else if index.is_some() && staked < COIN {
        state.get_stakers_mut().remove(index.unwrap()).unwrap();
    }
}
fn update_0<T: State>(state: &mut T, timestamp: u32, previous_timestamp: u32, loading: bool) {
    for n in 0..offline(timestamp, previous_timestamp) {
        let staker = state.staker_n(n as isize);
        if staker.is_none() {
            break;
        }
        let staker = staker.unwrap().clone();
        let mut staked = state.staked(&staker);
        let penalty = COIN * (n + 1) as u128;
        if !loading {
            warn!(
                "{} {} {}{}",
                "Slashed".red(),
                address::encode(&staker).green(),
                "-".yellow(),
                pea_int::to_string(penalty).yellow()
            );
        }
        staked = staked.saturating_sub(penalty);
        staked_insert(state, staker, staked);
        update_staker(state, staker);
    }
}
fn update_1<T: State>(state: &mut T, block: &BlockA) {
    let input_address = block.input_address();
    let mut balance = state.balance(&input_address);
    balance += block.reward();
    if let Some(stake) = block.stakes.first() {
        if stake.fee == 0 {
            staked_insert(state, input_address, COIN)
        }
    }
    balance_insert(state, input_address, balance)
}
fn update_2<T: State>(state: &mut T, block: &BlockA) {
    for transaction in block.transactions.iter() {
        let mut balance_input = state.balance(&transaction.input_address);
        let mut balance_output = state.balance(&transaction.output_address);
        balance_input -= transaction.amount + transaction.fee;
        balance_output += transaction.amount;
        balance_insert(state, transaction.input_address, balance_input);
        balance_insert(state, transaction.output_address, balance_output);
    }
    for stake in block.stakes.iter() {
        let mut balance = state.balance(&stake.input_address);
        let mut staked = state.staked(&stake.input_address);
        if stake.deposit {
            balance -= stake.amount + stake.fee;
            staked += stake.amount;
        } else {
            balance += stake.amount - stake.fee;
            staked -= stake.amount;
        }
        balance_insert(state, stake.input_address, balance);
        staked_insert(state, stake.input_address, staked);
    }
}
fn update_3<T: State>(state: &mut T, block: &BlockA) {
    for stake in block.stakes.iter() {
        update_staker(state, stake.input_address);
    }
}
pub fn update<T: State>(state: &mut T, block: &BlockA, previous_timestamp: u32, loading: bool) {
    update_0(state, block.timestamp, previous_timestamp, loading);
    update_1(state, block);
    update_2(state, block);
    update_3(state, block);
}
pub fn append_block<T: State>(state: &mut T, db: &DBWithThreadMode<SingleThreaded>, block: &BlockA, previous_timestamp: u32, loading: bool) {
    state.get_hashes_mut().push(block.hash);
    update(state, block, previous_timestamp, loading);
    *state.get_latest_block_mut() = db::block::get_a(db, &block.hash).unwrap();
}
pub fn load<T: State>(state: &mut T, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
    let mut previous_timestamp = match hashes.first() {
        Some(hash) => db::block::get_b(db, hash).unwrap().timestamp,
        None => 0,
    };
    for hash in hashes.iter() {
        let block_a = db::block::get_a(db, hash).unwrap();
        state.append_block(db, &block_a, previous_timestamp, true);
        previous_timestamp = block_a.timestamp;
    }
}
pub fn offline(timestamp: u32, previous_timestamp: u32) -> usize {
    let diff = timestamp.saturating_sub(previous_timestamp + 1);
    (diff / BLOCK_TIME_MAX as u32) as usize
}
fn staker_n<T: State>(state: &T, n: isize) -> Option<AddressBytes> {
    type StakedMap = HashMap<AddressBytes, u128>;
    fn winner(beta: &Beta, staked_map: &StakedMap, n: u128, m: u128) -> (AddressBytes, u128) {
        let random = util::random(beta, n as u128, m);
        let mut counter = 0;
        for (staker, staked) in staked_map.iter() {
            counter += staked;
            if random <= counter {
                return (*staker, *staked);
            }
        }
        unreachable!()
    }
    if n < 0 {
        return None;
    }
    let n = n.abs() as usize;
    let mut m = 0;
    let mut staked_map: StakedMap = HashMap::new();
    for staker in state.get_stakers().iter() {
        let staked = state.staked(staker);
        m += staked;
        staked_map.insert(staker.clone(), staked);
    }
    for _n in 0..(n + 1) {
        let penalty = COIN * _n as u128;
        m = m.saturating_sub(penalty);
        if m == 0 {
            return None;
        }
        let (staker, staked) = winner(&state.get_latest_block().beta, &staked_map, _n as u128, m);
        staked_map.insert(staker.clone(), staked.saturating_sub(penalty));
        if _n == n {
            return Some(staker);
        }
    }
    unreachable!()
}
