use colored::*;
use log::debug;
use pea_address as address;
use pea_block::Block;
use pea_core::{constants::BLOCK_TIME_MAX, constants::MIN_STAKE, types, util};
use pea_db as db;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::collections::{HashMap, VecDeque};
macro_rules! impl_State {
    (for $($t:ty),+) => {
        $(impl $t {
            pub fn balance(&self, public_key: &types::PublicKeyBytes) -> u128 {
                match self.balance.get(public_key) {
                    Some(b) => *b,
                    None => 0,
                }
            }
            pub fn balance_staked(&self, public_key: &types::PublicKeyBytes) -> u128 {
                match self.balance_staked.get(public_key) {
                    Some(b) => *b,
                    None => 0,
                }
            }
            fn update_balances(&mut self, block: &Block) {
                for transaction in block.transactions.iter() {
                    let mut balance_input = self.balance(&transaction.public_key_input);
                    let mut balance_output = self.balance(&transaction.public_key_output);
                    balance_input -= transaction.amount + transaction.fee;
                    balance_output += transaction.amount;
                    self.balance.insert(transaction.public_key_input, balance_input);
                    self.balance.insert(transaction.public_key_output, balance_output);
                }
                for stake in block.stakes.iter() {
                    let mut balance = self.balance(&stake.public_key);
                    let mut balance_staked = self.balance_staked(&stake.public_key);
                    if stake.deposit {
                        balance -= stake.amount + stake.fee;
                        balance_staked += stake.amount;
                    } else {
                        balance += stake.amount - stake.fee;
                        balance_staked -= stake.amount;
                    }
                    self.balance.insert(stake.public_key, balance);
                    self.balance_staked.insert(stake.public_key, balance_staked);
                }
            }
            fn update_stakers(&mut self, block: &Block) {
                if self.stakers.len() > 1 {
                    self.stakers.rotate_left(1);
                }
                for stake in block.stakes.iter() {
                    let balance_staked = self.balance_staked(&stake.public_key);
                    let any = self.stakers.iter().any(|x| x == &stake.public_key);
                    if !any && balance_staked >= MIN_STAKE {
                        self.stakers.push_back(stake.public_key);
                    } else if any && balance_staked < MIN_STAKE {
                        self.balance_staked.remove(&stake.public_key);
                        let index = self
                            .stakers
                            .iter()
                            .position(|x| x == &stake.public_key)
                            .unwrap();
                        self.stakers.remove(index).unwrap();
                        debug!(
                            "{} {}",
                            "Burned low balance".red(),
                            address::public::encode(&stake.public_key),
                        );
                    }
                }
            }
            fn update_reward(&mut self, block: &Block) {
                let balance_staked = self.balance_staked(&block.public_key);
                let mut balance = self.balance(&block.public_key);
                balance += block.reward(balance_staked);
                if let Some(stake) = block.stakes.first() {
                    if stake.fee == 0 {
                        balance += MIN_STAKE;
                        debug!(
                            "{} {} {} {}",
                            "Minted".cyan(),
                            MIN_STAKE.to_string().yellow(),
                            address::public::encode(&block.public_key).green(),
                            hex::encode(block.hash())
                        );
                    }
                }
                self.balance.insert(block.public_key, balance);
            }
            fn staker_index(timestamp: u32, previous_timestamp: u32) -> usize {
                let diff = timestamp.saturating_sub(previous_timestamp + 1);
                let index = diff / BLOCK_TIME_MAX as u32;
                index as usize
            }
            fn update_penalty(
                &mut self,
                timestamp: u32,
                previous_timestamp: u32,
            ) {
                for _ in 0..Self::staker_index(timestamp, previous_timestamp) {
                    if self.stakers.is_empty() {
                        break;
                    }
                    self.balance_staked.remove(self.stakers.get(0).unwrap());
                    self.stakers.remove(0).unwrap();
                }
            }
            pub fn update(&mut self, block: &Block, previous_timestamp: u32) {
                self.hashes.push(block.hash());
                self.update_penalty(block.timestamp, previous_timestamp);
                self.update_reward(block);
                self.update_balances(block);
                self.update_stakers(block);
            }
            pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &[types::Hash]) {
                let mut previous_timestamp = match hashes.first() {
                    Some(hash) => db::block::get(db, hash).unwrap().timestamp,
                    None => 0,
                };
                for hash in hashes.iter() {
                    let block = db::block::get(db, hash).unwrap();
                    self.update(&block, previous_timestamp);
                    previous_timestamp = block.timestamp;
                }
            }
        })*
    }
}
impl_State!(for Dynamic, Trusted);
#[derive(Debug, Clone)]
pub struct Dynamic {
    pub hashes: Vec<types::Hash>,
    pub stakers: VecDeque<types::PublicKeyBytes>,
    balance: HashMap<types::PublicKeyBytes, u128>,
    balance_staked: HashMap<types::PublicKeyBytes, u128>,
    pub latest_block: Block,
}
impl Dynamic {
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
        self.stakers.get(Self::staker_index(timestamp, previous_timestamp))
    }
    pub fn current_staker(&self) -> Option<&types::PublicKeyBytes> {
        self.staker(util::timestamp(), self.latest_block.timestamp)
    }
    pub fn offline_staker(&self) -> Option<&types::PublicKeyBytes> {
        let index = Self::staker_index(util::timestamp(), self.latest_block.timestamp);
        if index == 0 {
            return None;
        }
        self.stakers.get(index - 1)
    }
}
impl Default for Dynamic {
    fn default() -> Self {
        Self {
            hashes: vec![],
            stakers: VecDeque::new(),
            balance: HashMap::new(),
            balance_staked: HashMap::new(),
            latest_block: Block::default(),
        }
    }
}
#[derive(Default, Debug, Clone)]
pub struct Trusted {
    pub hashes: Vec<types::Hash>,
    pub stakers: VecDeque<types::PublicKeyBytes>,
    balance: HashMap<types::PublicKeyBytes, u128>,
    balance_staked: HashMap<types::PublicKeyBytes, u128>,
}
