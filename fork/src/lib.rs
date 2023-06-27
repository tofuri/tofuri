use address::public;
use block::Block;
use db::checkpoint::CheckpointDB;
use decimal::Decimal;
use rocksdb::DB;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use stake::Stake;
use std::collections::HashMap;
use std::collections::VecDeque;
use tracing::debug;
use tracing::warn;
use transaction::Transaction;
use tree::Tree;
use tree::GENESIS_BLOCK_PREVIOUS_HASH;
use uint::construct_uint;
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
        Some(hash) => db::block::get(db, hash).unwrap().timestamp,
        None => 0,
    };
    for hash in hashes.iter() {
        let block = db::block::get(db, hash).unwrap();
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Manager {
    pub stable: Stable,
    pub unstable: Unstable,
}
impl Manager {
    pub fn unstable(
        &self,
        db: &DB,
        tree: &Tree,
        trust_fork_after_blocks: usize,
        previous_hash: &[u8; 32],
    ) -> Result<Unstable, Error> {
        if previous_hash == &GENESIS_BLOCK_PREVIOUS_HASH {
            let unstable = Unstable::default();
            return Ok(unstable);
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
            return Err(Error::NotAllowedToForkStableChain);
        }
        if let Some(hash) = hashes.last() {
            if hash == &GENESIS_BLOCK_PREVIOUS_HASH {
                hashes.pop();
            }
        }
        hashes.reverse();
        let unstable = Unstable::from(db, &hashes, &self.stable);
        Ok(unstable)
    }
    pub fn update(&mut self, db: &DB, hashes_1: &[[u8; 32]], trust_fork_after_blocks: usize) {
        let hashes_0 = &self.unstable.hashes;
        if hashes_0.len() == trust_fork_after_blocks {
            let block = db::block::get(db, hashes_0.first().unwrap()).unwrap();
            self.stable.append_block(
                &block,
                match db::block::get(db, &block.previous_hash) {
                    Ok(block) => block.timestamp,
                    Err(_) => 0,
                },
            );
        }
        self.unstable = Unstable::from(db, hashes_1, &self.stable);
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stable {
    pub latest_block: Block,
    pub hashes: Vec<[u8; 32]>,
    pub stakers: VecDeque<[u8; 20]>,
    latest_blocks: Vec<Block>,
    map_balance: HashMap<[u8; 20], u128>,
    map_staked: HashMap<[u8; 20], u128>,
}
impl Stable {
    pub fn append_block(&mut self, block: &Block, previous_timestamp: u32) {
        append_block(self, block, previous_timestamp, false)
    }
    pub fn load(&mut self, db: &DB, hashes: &[[u8; 32]]) {
        load(self, db, hashes)
    }
    pub fn checkpoint(&self) -> CheckpointDB {
        CheckpointDB {
            height: self.hashes.len(),
            latest_block: self.latest_block.clone(),
            stakers: self.stakers.clone(),
            latest_blocks: self.latest_blocks.clone(),
            map_balance: self.map_balance.clone(),
            map_staked: self.map_staked.clone(),
        }
    }
    pub fn from_checkpoint(hashes: Vec<[u8; 32]>, checkpoint: CheckpointDB) -> Stable {
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
    fn get_hashes_mut(&mut self) -> &mut Vec<[u8; 32]> {
        &mut self.hashes
    }
    fn get_stakers(&self) -> &VecDeque<[u8; 20]> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<[u8; 20]> {
        &mut self.stakers
    }
    fn get_map_balance(&self) -> &HashMap<[u8; 20], u128> {
        &self.map_balance
    }
    fn get_map_balance_mut(&mut self) -> &mut HashMap<[u8; 20], u128> {
        &mut self.map_balance
    }
    fn get_map_staked(&self) -> &HashMap<[u8; 20], u128> {
        &self.map_staked
    }
    fn get_map_staked_mut(&mut self) -> &mut HashMap<[u8; 20], u128> {
        &mut self.map_staked
    }
    fn get_latest_block(&self) -> &Block {
        &self.latest_block
    }
    fn get_latest_block_mut(&mut self) -> &mut Block {
        &mut self.latest_block
    }
    fn get_latest_blocks(&self) -> &Vec<Block> {
        &self.latest_blocks
    }
    fn get_latest_blocks_mut(&mut self) -> &mut Vec<Block> {
        &mut self.latest_blocks
    }
    fn is_stable() -> bool {
        true
    }
    fn append_block(&mut self, block: &Block, previous_timestamp: u32, loading: bool) {
        append_block(self, block, previous_timestamp, loading)
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Unstable {
    pub latest_block: Block,
    pub hashes: Vec<[u8; 32]>,
    pub stakers: VecDeque<[u8; 20]>,
    latest_blocks: Vec<Block>,
    map_balance: HashMap<[u8; 20], u128>,
    map_staked: HashMap<[u8; 20], u128>,
}
impl Unstable {
    pub fn from(db: &DB, hashes: &[[u8; 32]], stable: &Stable) -> Unstable {
        let mut unstable = Unstable {
            hashes: vec![],
            stakers: stable.stakers.clone(),
            map_balance: stable.get_map_balance().clone(),
            map_staked: stable.get_map_staked().clone(),
            latest_block: Block::default(),
            latest_blocks: stable.get_latest_blocks().clone(),
        };
        load(&mut unstable, db, hashes);
        unstable
    }
    pub fn check_overflow(
        &self,
        transactions: &Vec<Transaction>,
        stakes: &Vec<Stake>,
    ) -> Result<(), Error> {
        let mut map_balance: HashMap<[u8; 20], u128> = HashMap::new();
        let mut map_staked: HashMap<[u8; 20], u128> = HashMap::new();
        for transaction in transactions {
            let k = transaction.input_address().unwrap();
            let mut balance = if map_balance.contains_key(&k) {
                *map_balance.get(&k).unwrap()
            } else {
                self.balance(&k)
            };
            balance = balance
                .checked_sub(u128::from(transaction.amount + transaction.fee))
                .ok_or(Error::Overflow)?;
            map_balance.insert(k, balance);
        }
        for stake in stakes {
            let k = stake.input_address().unwrap();
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
            if stake.deposit {
                balance = balance
                    .checked_sub((stake.amount + stake.fee).into())
                    .ok_or(Error::Overflow)?;
            } else {
                balance = balance
                    .checked_sub(stake.fee.into())
                    .ok_or(Error::Overflow)?;
                staked = staked
                    .checked_sub(stake.amount.into())
                    .ok_or(Error::Overflow)?;
            }
            map_balance.insert(k, balance);
            map_staked.insert(k, staked);
        }
        Ok(())
    }
    pub fn transaction_in_chain(&self, transaction: &Transaction) -> bool {
        for block in self.latest_blocks.iter() {
            if block
                .transactions
                .iter()
                .any(|a| a.hash() == transaction.hash())
            {
                return true;
            }
        }
        false
    }
    pub fn stake_in_chain(&self, stake: &Stake) -> bool {
        for block in self.latest_blocks.iter() {
            if block.stakes.iter().any(|a| a.hash() == stake.hash()) {
                return true;
            }
        }
        false
    }
    pub fn balance(&self, address: &[u8; 20]) -> u128 {
        get_balance(self, address)
    }
    pub fn staked(&self, address: &[u8; 20]) -> u128 {
        get_staked(self, address)
    }
    pub fn next_staker(&self, timestamp: u32) -> Option<[u8; 20]> {
        next_staker(self, timestamp)
    }
    pub fn stakers_offline(&self, timestamp: u32, previous_timestamp: u32) -> Vec<[u8; 20]> {
        stakers_offline(self, timestamp, previous_timestamp)
    }
    pub fn stakers_n(&self, n: usize) -> Vec<[u8; 20]> {
        stakers_n(self, n).0
    }
}
impl Fork for Unstable {
    fn get_hashes_mut(&mut self) -> &mut Vec<[u8; 32]> {
        &mut self.hashes
    }
    fn get_stakers(&self) -> &VecDeque<[u8; 20]> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<[u8; 20]> {
        &mut self.stakers
    }
    fn get_map_balance(&self) -> &HashMap<[u8; 20], u128> {
        &self.map_balance
    }
    fn get_map_balance_mut(&mut self) -> &mut HashMap<[u8; 20], u128> {
        &mut self.map_balance
    }
    fn get_map_staked(&self) -> &HashMap<[u8; 20], u128> {
        &self.map_staked
    }
    fn get_map_staked_mut(&mut self) -> &mut HashMap<[u8; 20], u128> {
        &mut self.map_staked
    }
    fn get_latest_block(&self) -> &Block {
        &self.latest_block
    }
    fn get_latest_block_mut(&mut self) -> &mut Block {
        &mut self.latest_block
    }
    fn get_latest_blocks(&self) -> &Vec<Block> {
        &self.latest_blocks
    }
    fn get_latest_blocks_mut(&mut self) -> &mut Vec<Block> {
        &mut self.latest_blocks
    }
    fn is_stable() -> bool {
        false
    }
    fn append_block(&mut self, block: &Block, previous_timestamp: u32, loading: bool) {
        append_block(self, block, previous_timestamp, loading)
    }
}
