use crate::db;
use colored::*;
use log::debug;
use pea_address as address;
use pea_core::{block::Block, constants::BLOCK_TIME_MAX, constants::MIN_STAKE, types};
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::collections::{HashMap, VecDeque};
macro_rules! impl_State {
    (for $($t:ty),+) => {
        $(impl $t {
            pub fn balance(&self, public_key: &types::PublicKeyBytes) -> types::Amount {
                match self.balance.get(public_key) {
                    Some(b) => *b,
                    None => 0,
                }
            }
            pub fn balance_staked(&self, public_key: &types::PublicKeyBytes) -> types::Amount {
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
            fn update_penalty(
                &mut self,
                timestamp: &types::Timestamp,
                previous_timestamp: &types::Timestamp,
            ) {
                let mut diff = timestamp - previous_timestamp;
                if diff > 0 {
                    diff -= 1;
                }
                for _ in 0..diff / BLOCK_TIME_MAX as u32 {
                    if self.stakers.is_empty() {
                        break;
                    }
                    self.balance_staked.remove(self.stakers.get(0).unwrap());
                    self.stakers.remove(0).unwrap();
                }
            }
            pub fn update(&mut self, block: &Block, previous_timestamp: types::Timestamp) {
                self.hashes.push(block.hash());
                self.update_penalty(&block.timestamp, &previous_timestamp);
                self.update_reward(block);
                self.update_balances(block);
                self.update_stakers(block);
            }
            pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &Vec<types::Hash>) {
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
    pub hashes: types::Hashes,
    pub stakers: types::Stakers,
    balance: types::Balance,
    balance_staked: types::Balance,
    pub latest_block: Block,
}
impl Dynamic {
    pub fn from(
        db: &DBWithThreadMode<SingleThreaded>,
        hashes: &Vec<types::Hash>,
        trusted: &Trusted,
    ) -> Dynamic {
        let mut dynamic = Self {
            hashes: vec![],
            stakers: trusted.stakers.clone(),
            balance: trusted.balance.clone(),
            balance_staked: trusted.balance_staked.clone(),
            latest_block: Block::new_timestamp_0([0; 32]),
        };
        dynamic.load(db, hashes);
        match hashes.last() {
            Some(hash) => dynamic.latest_block = db::block::get(db, hash).unwrap(),
            None => {}
        };
        dynamic
    }
    pub fn staker(
        &self,
        timestamp: types::Timestamp,
        previous_timestamp: types::Timestamp,
    ) -> Option<&types::PublicKeyBytes> {
        let mut diff = timestamp - previous_timestamp;
        if diff > 0 {
            diff -= 1;
        }
        let index = diff / BLOCK_TIME_MAX as u32;
        self.stakers.get(index as usize)
    }
}
impl Default for Dynamic {
    fn default() -> Self {
        Self {
            hashes: vec![],
            stakers: VecDeque::new(),
            balance: HashMap::new(),
            balance_staked: HashMap::new(),
            latest_block: Block::new_timestamp_0([0; 32]),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Trusted {
    pub hashes: types::Hashes,
    pub stakers: types::Stakers,
    balance: types::Balance,
    balance_staked: types::Balance,
}
impl Default for Trusted {
    fn default() -> Self {
        Self {
            hashes: vec![],
            stakers: VecDeque::new(),
            balance: HashMap::new(),
            balance_staked: HashMap::new(),
        }
    }
}
