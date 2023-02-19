use colored::Colorize;
use log::warn;
use pea_address::address;
use pea_block::BlockA;
use pea_core::*;
use pea_db as db;
use pea_stake::StakeA;
use pea_transaction::TransactionA;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::error::Error;
pub type Map = HashMap<AddressBytes, u128>;
pub trait State {
    fn get_hashes_mut(&mut self) -> &mut Vec<Hash>;
    fn get_stakers(&self) -> &VecDeque<AddressBytes>;
    fn get_stakers_mut(&mut self) -> &mut VecDeque<AddressBytes>;
    fn get_map_balance(&self) -> &Map;
    fn get_map_balance_mut(&mut self) -> &mut Map;
    fn get_map_staked(&self) -> &Map;
    fn get_map_staked_mut(&mut self) -> &mut Map;
    fn get_latest_block(&self) -> &BlockA;
    fn get_latest_block_mut(&mut self) -> &mut BlockA;
    fn get_non_ancient_blocks(&self) -> &Vec<BlockA>;
    fn get_non_ancient_blocks_mut(&mut self) -> &mut Vec<BlockA>;
    fn is_trusted() -> bool;
    fn append_block(&mut self, block: &BlockA, previous_timestamp: u32, loading: bool);
    fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]);
}
#[derive(Default, Debug, Clone)]
pub struct Trusted {
    pub latest_block: BlockA,
    pub non_ancient_blocks: Vec<BlockA>,
    pub hashes: Vec<Hash>,
    pub stakers: VecDeque<AddressBytes>,
    map_balance: Map,
    map_staked: Map,
}
#[derive(Default, Debug, Clone)]
pub struct Dynamic {
    pub latest_block: BlockA,
    pub non_ancient_blocks: Vec<BlockA>,
    pub hashes: Vec<Hash>,
    pub stakers: VecDeque<AddressBytes>,
    map_balance: Map,
    map_staked: Map,
}
impl Trusted {
    pub fn append_block(&mut self, block: &BlockA, previous_timestamp: u32) {
        append_block(self, block, previous_timestamp, false)
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
        load(self, db, hashes)
    }
}
impl Dynamic {
    pub fn balance(&self, address: &AddressBytes) -> u128 {
        get_balance(self, address)
    }
    pub fn staked(&self, address: &AddressBytes) -> u128 {
        get_staked(self, address)
    }
    pub fn from(db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash], trusted: &Trusted) -> Dynamic {
        let mut dynamic = Self {
            hashes: vec![],
            stakers: trusted.stakers.clone(),
            map_balance: trusted.map_balance.clone(),
            map_staked: trusted.map_staked.clone(),
            latest_block: BlockA::default(),
            non_ancient_blocks: trusted.non_ancient_blocks.clone(),
        };
        dynamic.load(db, hashes);
        dynamic
    }
    pub fn next_staker(&self, timestamp: u32) -> Option<AddressBytes> {
        next_staker(self, timestamp)
    }
    pub fn stakers_offline(&self, timestamp: u32, previous_timestamp: u32) -> Vec<AddressBytes> {
        stakers_offline(self, timestamp, previous_timestamp)
    }
    pub fn stakers_n(&self, n: usize) -> Vec<AddressBytes> {
        stakers_n(self, n).0
    }
    pub fn check_overflow(&self, transactions: &Vec<TransactionA>, stakes: &Vec<StakeA>) -> Result<(), Box<dyn Error>> {
        let mut map_balance: HashMap<AddressBytes, u128> = HashMap::new();
        let mut map_staked: HashMap<AddressBytes, u128> = HashMap::new();
        for transaction_a in transactions {
            let k = transaction_a.input_address;
            let mut balance = if map_balance.contains_key(&k) {
                *map_balance.get(&k).unwrap()
            } else {
                self.balance(&k)
            };
            balance = balance.checked_sub(transaction_a.amount + transaction_a.fee).ok_or("overflow")?;
            map_balance.insert(k, balance);
        }
        for stake_a in stakes {
            let k = stake_a.input_address;
            let mut balance = if map_balance.contains_key(&k) {
                *map_balance.get(&k).unwrap()
            } else {
                self.balance(&k)
            };
            let mut staked = if map_staked.contains_key(&k) {
                *map_staked.get(&k).unwrap()
            } else {
                self.staked(&k)
            };
            if stake_a.deposit {
                balance = balance.checked_sub(stake_a.amount + stake_a.fee).ok_or("overflow")?;
            } else {
                balance = balance.checked_sub(stake_a.fee).ok_or("overflow")?;
                staked = staked.checked_sub(stake_a.amount).ok_or("overflow")?;
            }
            map_balance.insert(k, balance);
            map_staked.insert(k, staked);
        }
        Ok(())
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
    fn get_map_balance(&self) -> &Map {
        &self.map_balance
    }
    fn get_map_balance_mut(&mut self) -> &mut Map {
        &mut self.map_balance
    }
    fn get_map_staked(&self) -> &Map {
        &self.map_staked
    }
    fn get_map_staked_mut(&mut self) -> &mut Map {
        &mut self.map_staked
    }
    fn get_latest_block(&self) -> &BlockA {
        &self.latest_block
    }
    fn get_latest_block_mut(&mut self) -> &mut BlockA {
        &mut self.latest_block
    }
    fn get_non_ancient_blocks(&self) -> &Vec<BlockA> {
        &self.non_ancient_blocks
    }
    fn get_non_ancient_blocks_mut(&mut self) -> &mut Vec<BlockA> {
        &mut self.non_ancient_blocks
    }
    fn is_trusted() -> bool {
        true
    }
    fn append_block(&mut self, block: &BlockA, previous_timestamp: u32, loading: bool) {
        append_block(self, block, previous_timestamp, loading)
    }
    fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
        load(self, db, hashes)
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
    fn get_map_balance(&self) -> &Map {
        &self.map_balance
    }
    fn get_map_balance_mut(&mut self) -> &mut Map {
        &mut self.map_balance
    }
    fn get_map_staked(&self) -> &Map {
        &self.map_staked
    }
    fn get_map_staked_mut(&mut self) -> &mut Map {
        &mut self.map_staked
    }
    fn get_latest_block(&self) -> &BlockA {
        &self.latest_block
    }
    fn get_latest_block_mut(&mut self) -> &mut BlockA {
        &mut self.latest_block
    }
    fn get_non_ancient_blocks(&self) -> &Vec<BlockA> {
        &self.non_ancient_blocks
    }
    fn get_non_ancient_blocks_mut(&mut self) -> &mut Vec<BlockA> {
        &mut self.non_ancient_blocks
    }
    fn is_trusted() -> bool {
        false
    }
    fn append_block(&mut self, block: &BlockA, previous_timestamp: u32, loading: bool) {
        append_block(self, block, previous_timestamp, loading)
    }
    fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
        load(self, db, hashes)
    }
}
fn get_balance<T: State>(state: &T, address: &AddressBytes) -> u128 {
    match state.get_map_balance().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn get_staked<T: State>(state: &T, address: &AddressBytes) -> u128 {
    match state.get_map_staked().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn insert_balance<T: State>(state: &mut T, address: AddressBytes, balance: u128) {
    match balance {
        0 => state.get_map_balance_mut().remove(&address),
        x => state.get_map_balance_mut().insert(address, x),
    };
}
fn insert_staked<T: State>(state: &mut T, address: AddressBytes, staked: u128) {
    match staked {
        0 => state.get_map_staked_mut().remove(&address),
        x => state.get_map_staked_mut().insert(address, x),
    };
}
fn update_stakers<T: State>(state: &mut T, address: AddressBytes) {
    let staked = get_staked(state, &address);
    let index = state.get_stakers().iter().position(|x| x == &address);
    if index.is_none() && staked >= COIN {
        state.get_stakers_mut().push_back(address);
    } else if index.is_some() && staked < COIN {
        state.get_stakers_mut().remove(index.unwrap()).unwrap();
    }
}
fn update_0<T: State>(state: &mut T, timestamp: u32, previous_timestamp: u32, loading: bool) {
    let stakers = stakers_offline(state, timestamp, previous_timestamp);
    for (index, staker) in stakers.iter().enumerate() {
        let mut staked = get_staked(state, staker);
        let penalty = pea_util::penalty(index + 1);
        staked = staked.saturating_sub(penalty);
        insert_staked(state, *staker, staked);
        update_stakers(state, *staker);
        if !loading && !T::is_trusted() {
            warn!(
                "{} {} {}{}",
                "Slashed".red(),
                address::encode(staker).green(),
                "-".yellow(),
                pea_int::to_string(penalty).yellow()
            );
        }
    }
}
fn update_1<T: State>(state: &mut T, block: &BlockA) {
    let input_address = block.input_address();
    let mut balance = get_balance(state, &input_address);
    balance += block.reward();
    if let Some(stake) = block.stakes.first() {
        if stake.fee == 0 {
            insert_staked(state, input_address, COIN)
        }
    }
    insert_balance(state, input_address, balance)
}
fn update_2<T: State>(state: &mut T, block: &BlockA) {
    for transaction in block.transactions.iter() {
        let mut balance_input = get_balance(state, &transaction.input_address);
        let mut balance_output = get_balance(state, &transaction.output_address);
        balance_input -= transaction.amount + transaction.fee;
        balance_output += transaction.amount;
        insert_balance(state, transaction.input_address, balance_input);
        insert_balance(state, transaction.output_address, balance_output);
    }
    for stake in block.stakes.iter() {
        let mut balance = get_balance(state, &stake.input_address);
        let mut staked = get_staked(state, &stake.input_address);
        if stake.deposit {
            balance -= stake.amount + stake.fee;
            staked += stake.amount;
        } else {
            balance += stake.amount - stake.fee;
            staked -= stake.amount;
        }
        insert_balance(state, stake.input_address, balance);
        insert_staked(state, stake.input_address, staked);
    }
}
fn update_3<T: State>(state: &mut T, block: &BlockA) {
    for stake in block.stakes.iter() {
        update_stakers(state, stake.input_address);
    }
}
pub fn update<T: State>(state: &mut T, block: &BlockA, previous_timestamp: u32, loading: bool) {
    update_0(state, block.timestamp, previous_timestamp, loading);
    update_1(state, block);
    update_2(state, block);
    update_3(state, block);
}
fn update_non_ancient_blocks<T: State>(state: &mut T, block: &BlockA) {
    while state.get_non_ancient_blocks().first().is_some() && pea_util::ancient(state.get_non_ancient_blocks().first().unwrap().timestamp, block.timestamp) {
        (*state.get_non_ancient_blocks_mut()).remove(0);
    }
    (*state.get_non_ancient_blocks_mut()).push(block.clone());
}
pub fn append_block<T: State>(state: &mut T, block: &BlockA, previous_timestamp: u32, loading: bool) {
    state.get_hashes_mut().push(block.hash);
    update(state, block, previous_timestamp, loading);
    *state.get_latest_block_mut() = block.clone();
    update_non_ancient_blocks(state, block);
}
pub fn load<T: State>(state: &mut T, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
    let mut previous_timestamp = match hashes.first() {
        Some(hash) => db::block::get_b(db, hash).unwrap().timestamp,
        None => 0,
    };
    for hash in hashes.iter() {
        let block_a = db::block::get_a(db, hash).unwrap();
        state.append_block(&block_a, previous_timestamp, true);
        previous_timestamp = block_a.timestamp;
    }
}
fn stakers_n<T: State>(state: &T, n: usize) -> (Vec<AddressBytes>, bool) {
    fn random_n(slice: &[(AddressBytes, u128)], beta: &Beta, n: u128, modulo: u128) -> usize {
        let random = pea_util::random(beta, n, modulo);
        let mut counter = 0;
        for (index, (_, staked)) in slice.iter().enumerate() {
            counter += staked;
            if random <= counter {
                return index;
            }
        }
        unreachable!()
    }
    let mut modulo = 0;
    let mut vec: Vec<(AddressBytes, u128)> = vec![];
    for staker in state.get_stakers().iter() {
        let staked = get_staked(state, staker);
        modulo += staked;
        vec.push((*staker, staked));
    }
    vec.sort_by(|a, b| b.1.cmp(&a.1));
    let mut random_queue = vec![];
    for index in 0..(n + 1) {
        let penalty = pea_util::penalty(index);
        modulo = modulo.saturating_sub(penalty);
        if modulo == 0 {
            return (random_queue, true);
        }
        let index = random_n(&vec, &state.get_latest_block().beta, index as u128, modulo);
        vec[index] = (vec[index].0, vec[index].1.saturating_sub(penalty));
        random_queue.push(vec[index].0);
    }
    (random_queue, false)
}
fn offline(timestamp: u32, previous_timestamp: u32) -> usize {
    let diff = timestamp.saturating_sub(previous_timestamp + 1);
    (diff / BLOCK_TIME_MAX) as usize
}
pub fn next_staker<T: State>(state: &T, timestamp: u32) -> Option<AddressBytes> {
    match stakers_n(state, offline(timestamp, state.get_latest_block().timestamp)) {
        (_, true) => None,
        (x, _) => x.last().copied(),
    }
}
fn stakers_offline<T: State>(state: &T, timestamp: u32, previous_timestamp: u32) -> Vec<AddressBytes> {
    match offline(timestamp, previous_timestamp) {
        0 => vec![],
        n => stakers_n(state, n - 1).0,
    }
}
