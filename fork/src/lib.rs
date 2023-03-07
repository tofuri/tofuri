use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::error::Error;
use tofuri_address::address;
use tofuri_block::BlockA;
use tofuri_core::*;
use tofuri_stake::StakeA;
use tofuri_transaction::TransactionA;
use tofuri_tree::Tree;
use tracing::warn;
pub trait Fork {
    fn get_hashes_mut(&mut self) -> &mut Vec<Hash>;
    fn get_stakers(&self) -> &VecDeque<AddressBytes>;
    fn get_stakers_mut(&mut self) -> &mut VecDeque<AddressBytes>;
    fn get_map_balance(&self) -> &HashMap<AddressBytes, u128>;
    fn get_map_balance_mut(&mut self) -> &mut HashMap<AddressBytes, u128>;
    fn get_map_staked(&self) -> &HashMap<AddressBytes, u128>;
    fn get_map_staked_mut(&mut self) -> &mut HashMap<AddressBytes, u128>;
    fn get_latest_block(&self) -> &BlockA;
    fn get_latest_block_mut(&mut self) -> &mut BlockA;
    fn get_non_ancient_blocks(&self) -> &Vec<BlockA>;
    fn get_non_ancient_blocks_mut(&mut self) -> &mut Vec<BlockA>;
    fn is_stable() -> bool;
    fn append_block(&mut self, block_a: &BlockA, previous_timestamp: u32, loading: bool);
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
    fn get_non_ancient_blocks(&self) -> &Vec<BlockA> {
        &self.non_ancient_blocks
    }
    fn get_non_ancient_blocks_mut(&mut self) -> &mut Vec<BlockA> {
        &mut self.non_ancient_blocks
    }
    fn is_stable() -> bool {
        true
    }
    fn append_block(&mut self, block_a: &BlockA, previous_timestamp: u32, loading: bool) {
        append_block(self, block_a, previous_timestamp, loading)
    }
}
impl Fork for Unstable {
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
    fn get_non_ancient_blocks(&self) -> &Vec<BlockA> {
        &self.non_ancient_blocks
    }
    fn get_non_ancient_blocks_mut(&mut self) -> &mut Vec<BlockA> {
        &mut self.non_ancient_blocks
    }
    fn is_stable() -> bool {
        false
    }
    fn append_block(&mut self, block_a: &BlockA, previous_timestamp: u32, loading: bool) {
        append_block(self, block_a, previous_timestamp, loading)
    }
}
#[derive(Default, Debug, Clone)]
pub struct Manager {
    pub stable: Stable,
    pub unstable: Unstable,
}
#[derive(Default, Debug, Clone)]
pub struct Stable {
    pub latest_block: BlockA,
    pub hashes: Vec<Hash>,
    pub stakers: VecDeque<AddressBytes>,
    non_ancient_blocks: Vec<BlockA>,
    map_balance: HashMap<AddressBytes, u128>,
    map_staked: HashMap<AddressBytes, u128>,
}
#[derive(Default, Debug, Clone)]
pub struct Unstable {
    pub latest_block: BlockA,
    pub hashes: Vec<Hash>,
    pub stakers: VecDeque<AddressBytes>,
    non_ancient_blocks: Vec<BlockA>,
    map_balance: HashMap<AddressBytes, u128>,
    map_staked: HashMap<AddressBytes, u128>,
}
impl Manager {
    pub fn unstable(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        tree: &Tree,
        trust_fork_after_blocks: usize,
        previous_hash: &Hash,
    ) -> Result<Unstable, Box<dyn Error>> {
        if previous_hash == &GENESIS_BLOCK_PREVIOUS_HASH {
            return Ok(Unstable::default());
        }
        let first = self.unstable.hashes.first().unwrap();
        let mut hashes = vec![];
        let mut hash = *previous_hash;
        for _ in 0..trust_fork_after_blocks {
            hashes.push(hash);
            if first == &hash {
                break;
            }
            match tree.get(&hash) {
                Some(previous_hash) => hash = *previous_hash,
                None => break,
            };
        }
        if first != &hash && hash != GENESIS_BLOCK_PREVIOUS_HASH {
            return Err("not allowed to fork stable chain".into());
        }
        if let Some(hash) = hashes.last() {
            if hash == &GENESIS_BLOCK_PREVIOUS_HASH {
                hashes.pop();
            }
        }
        hashes.reverse();
        Ok(Unstable::from(db, &hashes, &self.stable))
    }
    pub fn update(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes_1: &[Hash], trust_fork_after_blocks: usize) {
        let hashes_0 = &self.unstable.hashes;
        if hashes_0.len() == trust_fork_after_blocks {
            let block_a = tofuri_db::block::get_a(db, hashes_0.first().unwrap()).unwrap();
            self.stable.append_block(
                &block_a,
                match tofuri_db::block::get_b(db, &block_a.previous_hash) {
                    Ok(block_b) => block_b.timestamp,
                    Err(_) => 0,
                },
            );
        }
        self.unstable = Unstable::from(db, hashes_1, &self.stable);
    }
}
impl Stable {
    pub fn append_block(&mut self, block_a: &BlockA, previous_timestamp: u32) {
        append_block(self, block_a, previous_timestamp, false)
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
        load(self, db, hashes)
    }
}
impl Unstable {
    pub fn from(db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash], stable: &Stable) -> Unstable {
        let mut unstable = Unstable {
            hashes: vec![],
            stakers: stable.stakers.clone(),
            map_balance: stable.map_balance.clone(),
            map_staked: stable.map_staked.clone(),
            latest_block: BlockA::default(),
            non_ancient_blocks: stable.non_ancient_blocks.clone(),
        };
        load(&mut unstable, db, hashes);
        unstable
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
    pub fn transaction_in_chain(&self, transaction_a: &TransactionA) -> bool {
        for block_a in self.non_ancient_blocks.iter() {
            if block_a.transactions.iter().any(|a| a.hash == transaction_a.hash) {
                return true;
            }
        }
        false
    }
    pub fn stake_in_chain(&self, stake_a: &StakeA) -> bool {
        for block_a in self.non_ancient_blocks.iter() {
            if block_a.stakes.iter().any(|a| a.hash == stake_a.hash) {
                return true;
            }
        }
        false
    }
    pub fn balance(&self, address: &AddressBytes) -> u128 {
        get_balance(self, address)
    }
    pub fn staked(&self, address: &AddressBytes) -> u128 {
        get_staked(self, address)
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
}
fn get_balance<T: Fork>(fork: &T, address: &AddressBytes) -> u128 {
    match fork.get_map_balance().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn get_staked<T: Fork>(fork: &T, address: &AddressBytes) -> u128 {
    match fork.get_map_staked().get(address) {
        Some(b) => *b,
        None => 0,
    }
}
fn insert_balance<T: Fork>(fork: &mut T, address: AddressBytes, balance: u128) {
    match balance {
        0 => fork.get_map_balance_mut().remove(&address),
        x => fork.get_map_balance_mut().insert(address, x),
    };
}
fn insert_staked<T: Fork>(fork: &mut T, address: AddressBytes, staked: u128) {
    match staked {
        0 => fork.get_map_staked_mut().remove(&address),
        x => fork.get_map_staked_mut().insert(address, x),
    };
}
fn update_stakers<T: Fork>(fork: &mut T, address: AddressBytes) {
    let staked = get_staked(fork, &address);
    let index = fork.get_stakers().iter().position(|x| x == &address);
    if index.is_none() && staked >= COIN {
        fork.get_stakers_mut().push_back(address);
    } else if index.is_some() && staked < COIN {
        fork.get_stakers_mut().remove(index.unwrap()).unwrap();
    }
}
fn update_0<T: Fork>(fork: &mut T, block_a: &BlockA, previous_timestamp: u32, loading: bool) {
    let stakers = stakers_offline(fork, block_a.timestamp, previous_timestamp);
    for (index, staker) in stakers.iter().enumerate() {
        let mut staked = get_staked(fork, staker);
        let penalty = tofuri_util::penalty(index + 1);
        staked = staked.saturating_sub(penalty);
        insert_staked(fork, *staker, staked);
        update_stakers(fork, *staker);
        if !loading && !T::is_stable() {
            warn!(amount = tofuri_int::to_string(penalty), address = address::encode(staker), "Slashed");
        }
    }
    if stakers_n(fork, offline(block_a.timestamp, previous_timestamp)).1 {
        let input_address = block_a.input_address();
        insert_staked(fork, input_address, COIN);
        update_stakers(fork, input_address);
        if !loading && !T::is_stable() {
            warn!(amount = tofuri_int::to_string(COIN), address = address::encode(&input_address), "Minted",)
        }
    }
}
fn update_1<T: Fork>(fork: &mut T, block_a: &BlockA) {
    let input_address = block_a.input_address();
    let mut balance = get_balance(fork, &input_address);
    balance += block_a.reward();
    insert_balance(fork, input_address, balance)
}
fn update_2<T: Fork>(fork: &mut T, block_a: &BlockA) {
    for transaction in block_a.transactions.iter() {
        let mut balance_input = get_balance(fork, &transaction.input_address);
        let mut balance_output = get_balance(fork, &transaction.output_address);
        balance_input -= transaction.amount + transaction.fee;
        balance_output += transaction.amount;
        insert_balance(fork, transaction.input_address, balance_input);
        insert_balance(fork, transaction.output_address, balance_output);
    }
    for stake in block_a.stakes.iter() {
        let mut balance = get_balance(fork, &stake.input_address);
        let mut staked = get_staked(fork, &stake.input_address);
        if stake.deposit {
            balance -= stake.amount + stake.fee;
            staked += stake.amount;
        } else {
            balance += stake.amount - stake.fee;
            staked -= stake.amount;
        }
        insert_balance(fork, stake.input_address, balance);
        insert_staked(fork, stake.input_address, staked);
    }
}
fn update_3<T: Fork>(fork: &mut T, block_a: &BlockA) {
    for stake in block_a.stakes.iter() {
        update_stakers(fork, stake.input_address);
    }
}
pub fn update<T: Fork>(fork: &mut T, block_a: &BlockA, previous_timestamp: u32, loading: bool) {
    update_0(fork, block_a, previous_timestamp, loading);
    update_1(fork, block_a);
    update_2(fork, block_a);
    update_3(fork, block_a);
}
fn update_non_ancient_blocks<T: Fork>(fork: &mut T, block_a: &BlockA) {
    while fork.get_non_ancient_blocks().first().is_some() && tofuri_util::ancient(fork.get_non_ancient_blocks().first().unwrap().timestamp, block_a.timestamp) {
        (*fork.get_non_ancient_blocks_mut()).remove(0);
    }
    (*fork.get_non_ancient_blocks_mut()).push(block_a.clone());
}
pub fn append_block<T: Fork>(fork: &mut T, block_a: &BlockA, previous_timestamp: u32, loading: bool) {
    update(fork, block_a, previous_timestamp, loading);
    update_non_ancient_blocks(fork, block_a);
    fork.get_hashes_mut().push(block_a.hash);
    *fork.get_latest_block_mut() = block_a.clone();
}
pub fn load<T: Fork>(fork: &mut T, db: &DBWithThreadMode<SingleThreaded>, hashes: &[Hash]) {
    let mut previous_timestamp = match hashes.first() {
        Some(hash) => tofuri_db::block::get_b(db, hash).unwrap().timestamp,
        None => 0,
    };
    for hash in hashes.iter() {
        let block_a = tofuri_db::block::get_a(db, hash).unwrap();
        fork.append_block(&block_a, previous_timestamp, T::is_stable());
        previous_timestamp = block_a.timestamp;
    }
}
fn stakers_n<T: Fork>(fork: &T, n: usize) -> (Vec<AddressBytes>, bool) {
    fn random_n(slice: &[(AddressBytes, u128)], beta: &Beta, n: u128, modulo: u128) -> usize {
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
    let mut vec: Vec<(AddressBytes, u128)> = vec![];
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
        let index = random_n(&vec, &fork.get_latest_block().beta, index as u128, modulo);
        vec[index] = (vec[index].0, vec[index].1.saturating_sub(penalty));
        random_queue.push(vec[index].0);
    }
    (random_queue, false)
}
fn offline(timestamp: u32, previous_timestamp: u32) -> usize {
    let diff = timestamp.saturating_sub(previous_timestamp + 1);
    (diff / BLOCK_TIME) as usize
}
pub fn next_staker<T: Fork>(fork: &T, timestamp: u32) -> Option<AddressBytes> {
    match stakers_n(fork, offline(timestamp, fork.get_latest_block().timestamp)) {
        (_, true) => None,
        (x, _) => x.last().copied(),
    }
}
fn stakers_offline<T: Fork>(fork: &T, timestamp: u32, previous_timestamp: u32) -> Vec<AddressBytes> {
    match offline(timestamp, previous_timestamp) {
        0 => vec![],
        n => stakers_n(fork, n - 1).0,
    }
}
