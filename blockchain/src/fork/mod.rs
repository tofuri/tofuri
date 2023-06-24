mod manager;
mod stable;
mod unstable;
use decimal::Decimal;
pub use manager::Manager;
use rocksdb::DB;
use sha2::Digest;
use sha2::Sha256;
pub use stable::Stable;
use std::collections::HashMap;
use std::collections::VecDeque;
use tofuri_address::public;
use tofuri_block::Block;
use tracing::debug;
use tracing::warn;
use uint::construct_uint;
pub use unstable::Unstable;
pub const BLOCK_TIME: u32 = 60;
pub const ELAPSED: u32 = 90;
#[derive(Debug)]
pub enum Error {
    NotAllowedToForkStableChain,
    Overflow,
}
pub trait Fork {
    fn get_hashes_mut(&mut self) -> &mut Vec<[u8; 32]>;
    fn get_stakers(&self) -> &VecDeque<[u8; 20]>;
    fn get_stakers_mut(&mut self) -> &mut VecDeque<[u8; 20]>;
    fn get_map_balance(&self) -> &HashMap<[u8; 20], u128>;
    fn get_map_balance_mut(&mut self) -> &mut HashMap<[u8; 20], u128>;
    fn get_map_staked(&self) -> &HashMap<[u8; 20], u128>;
    fn get_map_staked_mut(&mut self) -> &mut HashMap<[u8; 20], u128>;
    fn get_latest_block(&self) -> &Block;
    fn get_latest_block_mut(&mut self) -> &mut Block;
    fn get_latest_blocks(&self) -> &Vec<Block>;
    fn get_latest_blocks_mut(&mut self) -> &mut Vec<Block>;
    fn is_stable() -> bool;
    fn append_block(&mut self, block: &Block, previous_timestamp: u32, loading: bool);
}
fn get_balance<T: Fork>(fork: &T, address: &[u8; 20]) -> u128 {
    match fork.get_map_balance().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn get_staked<T: Fork>(fork: &T, address: &[u8; 20]) -> u128 {
    match fork.get_map_staked().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn insert_balance<T: Fork>(fork: &mut T, address: [u8; 20], balance: u128) {
    match balance {
        0 => fork.get_map_balance_mut().remove(&address),
        x => fork.get_map_balance_mut().insert(address, x),
    };
}
fn insert_staked<T: Fork>(fork: &mut T, address: [u8; 20], staked: u128) {
    match staked {
        0 => fork.get_map_staked_mut().remove(&address),
        x => fork.get_map_staked_mut().insert(address, x),
    };
}
fn update_stakers<T: Fork>(fork: &mut T, address: [u8; 20]) {
    let staked = get_staked(fork, &address);
    let index = fork.get_stakers().iter().position(|x| x == &address);
    let threshold = 10_u128.pow(18) * (fork.get_stakers().len() + 1) as u128;
    if index.is_none() && staked >= threshold {
        fork.get_stakers_mut().push_back(address);
    } else if index.is_some() && staked < threshold {
        fork.get_stakers_mut().remove(index.unwrap()).unwrap();
    }
}
fn update_0<T: Fork>(fork: &mut T, block: &Block, previous_timestamp: u32, loading: bool) {
    let stakers = stakers_offline(fork, block.timestamp, previous_timestamp);
    for (index, staker) in stakers.iter().enumerate() {
        let mut staked = get_staked(fork, staker);
        let penalty = penalty(index + 1);
        staked = staked.saturating_sub(penalty);
        insert_staked(fork, *staker, staked);
        update_stakers(fork, *staker);
        if !loading && !T::is_stable() {
            warn!(
                amount = penalty.decimal::<18>(),
                address = public::encode(staker),
                "Slashed"
            );
        }
    }
    if stakers_n(fork, offline(block.timestamp, previous_timestamp)).1 {
        let input_address = block.input_address().unwrap();
        insert_staked(fork, input_address, 10_u128.pow(18));
        update_stakers(fork, input_address);
        let address = public::encode(&input_address);
        if !loading && !T::is_stable() {
            warn!(address, "Minted")
        }
        if loading {
            debug!(address, "Minted")
        }
    }
}
fn update_1<T: Fork>(fork: &mut T, block: &Block) {
    let input_address = block.input_address().unwrap();
    let mut balance = get_balance(fork, &input_address);
    balance += block.reward();
    insert_balance(fork, input_address, balance)
}
fn update_2<T: Fork>(fork: &mut T, block: &Block) {
    for transaction in block.transactions.iter() {
        let mut balance_input = get_balance(fork, &transaction.input_address().unwrap());
        let mut balance_output = get_balance(fork, &transaction.output_address);
        balance_input -= transaction.amount + transaction.fee;
        balance_output += transaction.amount;
        insert_balance(fork, transaction.input_address().unwrap(), balance_input);
        insert_balance(fork, transaction.output_address, balance_output);
    }
    for stake in block.stakes.iter() {
        let mut balance = get_balance(fork, &stake.input_address().unwrap());
        let mut staked = get_staked(fork, &stake.input_address().unwrap());
        if stake.deposit {
            balance -= stake.amount + stake.fee;
            staked += stake.amount;
        } else {
            balance += stake.amount - stake.fee;
            staked -= stake.amount;
        }
        insert_balance(fork, stake.input_address().unwrap(), balance);
        insert_staked(fork, stake.input_address().unwrap(), staked);
    }
}
fn update_3<T: Fork>(fork: &mut T, block: &Block) {
    for stake in block.stakes.iter() {
        update_stakers(fork, stake.input_address().unwrap());
    }
}
fn update<T: Fork>(fork: &mut T, block: &Block, previous_timestamp: u32, loading: bool) {
    update_0(fork, block, previous_timestamp, loading);
    update_1(fork, block);
    update_2(fork, block);
    update_3(fork, block);
}
fn update_latest_blocks<T: Fork>(fork: &mut T, block: &Block) {
    while fork.get_latest_blocks().first().is_some()
        && elapsed(
            fork.get_latest_blocks().first().unwrap().timestamp,
            block.timestamp,
        )
    {
        (*fork.get_latest_blocks_mut()).remove(0);
    }
    (*fork.get_latest_blocks_mut()).push(block.clone());
}
fn append_block<T: Fork>(fork: &mut T, block: &Block, previous_timestamp: u32, loading: bool) {
    update(fork, block, previous_timestamp, loading);
    update_latest_blocks(fork, block);
    fork.get_hashes_mut().push(block.hash());
    *fork.get_latest_block_mut() = block.clone();
}
fn load<T: Fork>(fork: &mut T, db: &DB, hashes: &[[u8; 32]]) {
    let mut previous_timestamp = match hashes.first() {
        Some(hash) => tofuri_db::block::get(db, hash).unwrap().timestamp,
        None => 0,
    };
    for hash in hashes.iter() {
        let block = tofuri_db::block::get(db, hash).unwrap();
        fork.append_block(&block, previous_timestamp, T::is_stable());
        previous_timestamp = block.timestamp;
    }
}
fn stakers_n<T: Fork>(fork: &T, n: usize) -> (Vec<[u8; 20]>, bool) {
    fn random_n(slice: &[([u8; 20], u128)], beta: &[u8; 32], n: u128, modulo: u128) -> usize {
        let random = random(beta, n, modulo);
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
    let mut vec: Vec<([u8; 20], u128)> = vec![];
    for staker in fork.get_stakers().iter() {
        let staked = get_staked(fork, staker);
        modulo += staked;
        vec.push((*staker, staked));
    }
    vec.sort_by(|a, b| b.1.cmp(&a.1));
    let mut random_queue = vec![];
    for index in 0..(n + 1) {
        let penalty = penalty(index);
        modulo = modulo.saturating_sub(penalty);
        if modulo == 0 {
            return (random_queue, true);
        }
        let index = random_n(
            &vec,
            &fork.get_latest_block().beta().unwrap(),
            index as u128,
            modulo,
        );
        vec[index] = (vec[index].0, vec[index].1.saturating_sub(penalty));
        random_queue.push(vec[index].0);
    }
    (random_queue, false)
}
fn offline(timestamp: u32, previous_timestamp: u32) -> usize {
    let diff = timestamp.saturating_sub(previous_timestamp + 1);
    (diff / BLOCK_TIME) as usize
}
fn next_staker<T: Fork>(fork: &T, timestamp: u32) -> Option<[u8; 20]> {
    match stakers_n(fork, offline(timestamp, fork.get_latest_block().timestamp)) {
        (_, true) => None,
        (x, _) => x.last().copied(),
    }
}
fn stakers_offline<T: Fork>(fork: &T, timestamp: u32, previous_timestamp: u32) -> Vec<[u8; 20]> {
    match offline(timestamp, previous_timestamp) {
        0 => vec![],
        n => stakers_n(fork, n - 1).0,
    }
}
construct_uint! {
    pub struct U256(4);
}
pub fn u256(hash: &[u8; 32]) -> U256 {
    U256::from_big_endian(hash)
}
pub fn u256_modulo(hash: &[u8; 32], modulo: u128) -> u128 {
    (u256(hash) % modulo).as_u128()
}
pub fn hash_beta_n(beta: &[u8; 32], n: u128) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(beta);
    hasher.update(n.to_be_bytes());
    hasher.finalize().into()
}
pub fn random(beta: &[u8; 32], n: u128, modulo: u128) -> u128 {
    u256_modulo(&hash_beta_n(beta, n), modulo)
}
pub fn elapsed(timestamp: u32, latest_block_timestamp: u32) -> bool {
    ELAPSED + timestamp < latest_block_timestamp
}
pub fn penalty(index: usize) -> u128 {
    if index == 0 {
        return 0;
    }
    10_u128.pow(18) * 2_u128.pow(index as u32 - 1)
}
