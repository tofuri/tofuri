use crate::{address, block::Block, constants::BLOCK_TIME_MAX, constants::MIN_STAKE, types, util};
use colored::*;
use log::warn;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::collections::{HashMap, VecDeque};
#[derive(Debug, Clone)]
pub struct State {
    hashes: types::Hashes,
    stakers: types::Stakers,
    balance: types::Balance,
    balance_staked: types::Balance,
    sum_stakes_current: types::Amount,
    sum_stakes_all_time: types::Amount,
    latest_block: Block,
}
impl State {
    pub fn new() -> State {
        State {
            hashes: vec![],
            stakers: VecDeque::new(),
            balance: HashMap::new(),
            balance_staked: HashMap::new(),
            sum_stakes_current: 0,
            sum_stakes_all_time: 0,
            latest_block: Block::new([0; 32]),
        }
    }
    pub fn get_stakers(&self) -> &types::Stakers {
        &self.stakers
    }
    pub fn get_hashes(&self) -> &types::Hashes {
        &self.hashes
    }
    pub fn get_latest_block(&self) -> &Block {
        &self.latest_block
    }
    pub fn get_sum_stakes_current(&self) -> &types::Amount {
        &self.sum_stakes_current
    }
    pub fn get_sum_stakes_all_time(&self) -> &types::Amount {
        &self.sum_stakes_all_time
    }
    pub fn get_balance(&self, public_key: &types::PublicKeyBytes) -> types::Amount {
        match self.balance.get(public_key) {
            Some(b) => *b,
            None => 0,
        }
    }
    pub fn get_balance_staked(&self, public_key: &types::PublicKeyBytes) -> types::Amount {
        match self.balance_staked.get(public_key) {
            Some(b) => *b,
            None => 0,
        }
    }
    fn set_sum_stakes(&mut self) {
        let mut sum = 0;
        for staker in self.stakers.iter() {
            sum += self.get_balance_staked(&staker.0);
        }
        self.sum_stakes_current = sum;
        self.sum_stakes_all_time += sum;
    }
    fn set_balance(&mut self, public_key: types::PublicKeyBytes, balance: types::Amount) {
        self.balance.insert(public_key, balance);
    }
    fn set_balance_staked(
        &mut self,
        public_key: types::PublicKeyBytes,
        balance_staked: types::Amount,
    ) {
        self.balance_staked.insert(public_key, balance_staked);
    }
    fn set_balances(&mut self, block: &Block) {
        for transaction in block.transactions.iter() {
            let mut balance_input = self.get_balance(&transaction.public_key_input);
            let mut balance_output = self.get_balance(&transaction.public_key_output);
            balance_input -= transaction.amount + transaction.fee;
            balance_output += transaction.amount;
            self.set_balance(transaction.public_key_input, balance_input);
            self.set_balance(transaction.public_key_output, balance_output);
        }
        for stake in block.stakes.iter() {
            let mut balance = self.get_balance(&stake.public_key);
            let mut balance_staked = self.get_balance_staked(&stake.public_key);
            if stake.deposit {
                balance -= stake.amount + stake.fee;
                balance_staked += stake.amount;
            } else {
                balance += stake.amount - stake.fee;
                balance_staked -= stake.amount;
            }
            self.set_balance(stake.public_key, balance);
            self.set_balance_staked(stake.public_key, balance_staked);
        }
    }
    fn set_stakers(&mut self, block: &Block, height: usize) {
        if self.stakers.len() > 1 {
            self.stakers.rotate_left(1);
        }
        for stake in block.stakes.iter() {
            let balance_staked = self.get_balance_staked(&stake.public_key);
            let any = self.stakers.iter().any(|&e| e.0 == stake.public_key);
            if !any && balance_staked >= MIN_STAKE {
                self.stakers.push_back((stake.public_key, height));
            } else if any && balance_staked < MIN_STAKE {
                self.balance_staked.remove(&stake.public_key);
                let index = self
                    .stakers
                    .iter()
                    .position(|staker| staker.0 == stake.public_key)
                    .unwrap();
                self.stakers.remove(index).unwrap();
                warn!(
                    "{} {}",
                    "Burned low balance".red(),
                    address::encode(&stake.public_key)
                );
            }
        }
    }
    fn set_reward(&mut self, block: &Block) {
        let balance_staked = self.get_balance_staked(&block.public_key);
        let mut balance = self.get_balance(&block.public_key);
        balance += block.reward(balance_staked);
        if let Some(stake) = block.stakes.first() {
            if stake.fee == 0 {
                balance += MIN_STAKE;
                warn!(
                    "{} {} {}",
                    "Minted".cyan(),
                    MIN_STAKE.to_string().yellow(),
                    address::encode(&block.public_key).green()
                );
            }
        }
        self.set_balance(block.public_key, balance);
    }
    fn set_penalty(&mut self, timestamp: &types::Timestamp, previous_timestamp: &types::Timestamp) {
        if timestamp == previous_timestamp {
            return;
        }
        let diff = timestamp - previous_timestamp - 1;
        for _ in 0..diff / BLOCK_TIME_MAX as u32 {
            if !self.stakers.is_empty() {
                let public_key = self.stakers[0].0;
                self.balance_staked.remove(&public_key);
                self.stakers.remove(0).unwrap();
            }
        }
    }
    fn set(&mut self, block: &Block, height: types::Height) {
        self.set_reward(block);
        self.set_balances(block);
        self.set_stakers(block, height);
        self.set_sum_stakes();
    }
    pub fn penalty(&mut self) {
        if let Some((public_key, _)) = self.stakers.remove(0) {
            self.balance_staked.remove(&public_key).unwrap();
            warn!("{} {:?}", "Penalty".red(), address::encode(&public_key));
        }
    }
    pub fn reload(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        hashes: Vec<types::Hash>,
        latest: bool,
    ) {
        self.latest_block = Block::new([0; 32]);
        self.stakers.clear();
        self.balance.clear();
        self.balance_staked.clear();
        if hashes.is_empty() {
            return;
        }
        self.latest_block = Block::get(db, hashes.last().unwrap()).unwrap();
        let mut previous_block_timestamp = match hashes.first() {
            Some(hash) => Block::get(db, hash).unwrap().timestamp - 1,
            None => 0,
        };
        for (height, hash) in hashes.iter().enumerate() {
            let block = Block::get(db, hash).unwrap();
            self.set_penalty(&block.timestamp, &previous_block_timestamp);
            self.set(&block, height);
            previous_block_timestamp = block.timestamp;
        }
        if latest {
            self.set_penalty(&util::timestamp(), &self.latest_block.timestamp.clone());
        }
        self.hashes = hashes;
    }
    pub fn append(&mut self, block: Block) {
        self.hashes.push(block.hash());
        self.set(&block, self.hashes.len() - 1);
        self.latest_block = block;
    }
}
impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
