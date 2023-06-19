mod manager;
mod stable;
mod unstable;
pub use manager::Manager;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
pub use stable::Stable;
use std::collections::HashMap;
use std::collections::VecDeque;
use tofuri_address::address;
use tofuri_block::Block;
use tofuri_util::BLOCK_TIME;
use tracing::debug;
use tracing::warn;
pub use unstable::Unstable;
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
    fn append_block(&mut self, block_a: &Block, previous_timestamp: u32, loading: bool);
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
fn update_0<T: Fork>(fork: &mut T, block_a: &Block, previous_timestamp: u32, loading: bool) {
    let stakers = stakers_offline(fork, block_a.timestamp, previous_timestamp);
    for (index, staker) in stakers.iter().enumerate() {
        let mut staked = get_staked(fork, staker);
        let penalty = tofuri_util::penalty(index + 1);
        staked = staked.saturating_sub(penalty);
        insert_staked(fork, *staker, staked);
        update_stakers(fork, *staker);
        if !loading && !T::is_stable() {
            warn!(
                amount = parseint::to_string::<18>(penalty),
                address = address::encode(staker),
                "Slashed"
            );
        }
    }
    if stakers_n(fork, offline(block_a.timestamp, previous_timestamp)).1 {
        let input_address = block_a.input_address().unwrap();
        insert_staked(fork, input_address, 10_u128.pow(18));
        update_stakers(fork, input_address);
        let address = address::encode(&input_address);
        if !loading && !T::is_stable() {
            warn!(address, "Minted")
        }
        if loading {
            debug!(address, "Minted")
        }
    }
}
fn update_1<T: Fork>(fork: &mut T, block_a: &Block) {
    let input_address = block_a.input_address().unwrap();
    let mut balance = get_balance(fork, &input_address);
    balance += block_a.reward();
    insert_balance(fork, input_address, balance)
}
fn update_2<T: Fork>(fork: &mut T, block_a: &Block) {
    for transaction in block_a.transactions.iter() {
        let mut balance_input = get_balance(fork, &transaction.input_address().unwrap());
        let mut balance_output = get_balance(fork, &transaction.output_address);
        balance_input -= transaction.amount + transaction.fee;
        balance_output += transaction.amount;
        insert_balance(fork, transaction.input_address().unwrap(), balance_input);
        insert_balance(fork, transaction.output_address, balance_output);
    }
    for stake in block_a.stakes.iter() {
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
fn update_3<T: Fork>(fork: &mut T, block_a: &Block) {
    for stake in block_a.stakes.iter() {
        update_stakers(fork, stake.input_address().unwrap());
    }
}
fn update<T: Fork>(fork: &mut T, block_a: &Block, previous_timestamp: u32, loading: bool) {
    update_0(fork, block_a, previous_timestamp, loading);
    update_1(fork, block_a);
    update_2(fork, block_a);
    update_3(fork, block_a);
}
fn update_latest_blocks<T: Fork>(fork: &mut T, block_a: &Block) {
    while fork.get_latest_blocks().first().is_some()
        && tofuri_util::elapsed(
            fork.get_latest_blocks().first().unwrap().timestamp,
            block_a.timestamp,
        )
    {
        (*fork.get_latest_blocks_mut()).remove(0);
    }
    (*fork.get_latest_blocks_mut()).push(block_a.clone());
}
fn append_block<T: Fork>(fork: &mut T, block_a: &Block, previous_timestamp: u32, loading: bool) {
    update(fork, block_a, previous_timestamp, loading);
    update_latest_blocks(fork, block_a);
    fork.get_hashes_mut().push(block_a.hash());
    *fork.get_latest_block_mut() = block_a.clone();
}
fn load<T: Fork>(fork: &mut T, db: &DBWithThreadMode<SingleThreaded>, hashes: &[[u8; 32]]) {
    let mut previous_timestamp = match hashes.first() {
        Some(hash) => tofuri_db::block::get(db, hash).unwrap().timestamp,
        None => 0,
    };
    for hash in hashes.iter() {
        let block_a = tofuri_db::block::get(db, hash).unwrap();
        fork.append_block(&block_a, previous_timestamp, T::is_stable());
        previous_timestamp = block_a.timestamp;
    }
}
fn stakers_n<T: Fork>(fork: &T, n: usize) -> (Vec<[u8; 20]>, bool) {
    fn random_n(slice: &[([u8; 20], u128)], beta: &[u8; 32], n: u128, modulo: u128) -> usize {
        let random = tofuri_util::random(beta, n, modulo);
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
        let penalty = tofuri_util::penalty(index);
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
