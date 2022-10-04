use crate::{address, block::Block, constants::BLOCK_TIME_MAX, constants::MIN_STAKE, types};
use colored::*;
use log::debug;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::collections::{HashMap, VecDeque};
#[derive(Debug, Clone)]
pub struct Dynamic {
    hashes: types::Hashes,
    stakers: types::Stakers,
    balance: types::Balance,
    balance_staked: types::Balance,
    latest_block: Block,
}
impl Dynamic {
    pub fn new() -> Self {
        Self {
            hashes: vec![],
            stakers: VecDeque::new(),
            balance: HashMap::new(),
            balance_staked: HashMap::new(),
            latest_block: Block::new_timestamp_0([0; 32]),
        }
    }
    pub fn from(trusted: &Trusted) -> Self {
        Self {
            hashes: vec![],
            stakers: trusted.stakers.clone(),
            balance: trusted.balance.clone(),
            balance_staked: trusted.balance_staked.clone(),
            latest_block: Block::new_timestamp_0([0; 32]),
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
    pub fn get_staker(
        &self,
        timestamp: types::Timestamp,
        previous_timestamp: types::Timestamp,
    ) -> Option<&types::PublicKeyBytes> {
        let mut diff = timestamp - previous_timestamp;
        if diff > 0 {
            diff -= 1;
        }
        let index = diff / BLOCK_TIME_MAX as u32;
        self.get_stakers().get(index as usize)
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
    fn set_stakers(&mut self, block: &Block) {
        if self.stakers.len() > 1 {
            self.stakers.rotate_left(1);
        }
        for stake in block.stakes.iter() {
            let balance_staked = self.get_balance_staked(&stake.public_key);
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
                    address::encode(&stake.public_key),
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
                debug!(
                    "{} {} {} {}",
                    "Minted".cyan(),
                    MIN_STAKE.to_string().yellow(),
                    address::encode(&block.public_key).green(),
                    hex::encode(block.hash())
                );
            }
        }
        self.set_balance(block.public_key, balance);
    }
    pub fn set_penalty(
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
    fn set(&mut self, block: &Block, previous_timestamp: types::Timestamp) {
        self.set_penalty(&block.timestamp, &previous_timestamp);
        self.set_reward(block);
        self.set_balances(block);
        self.set_stakers(block);
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &Vec<types::Hash>) {
        let mut previous_timestamp = match hashes.first() {
            Some(hash) => Block::get(db, hash).unwrap().timestamp,
            None => 0,
        };
        for hash in hashes.iter() {
            let block = Block::get(db, hash).unwrap();
            let t = block.timestamp;
            self.append(block, previous_timestamp);
            previous_timestamp = t;
        }
    }
    pub fn reload(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        hashes: &Vec<types::Hash>,
        trusted: &Trusted,
    ) {
        self.hashes = vec![];
        self.stakers = trusted.stakers.clone();
        self.balance = trusted.balance.clone();
        self.balance_staked = trusted.balance_staked.clone();
        self.latest_block = Block::new_timestamp_0([0; 32]);
        self.load(db, hashes);
        // self.latest_block = Block::get(db, self.hashes.last().unwrap()).unwrap();
    }
    pub fn append(&mut self, block: Block, previous_timestamp: types::Timestamp) {
        self.hashes.push(block.hash());
        self.set(&block, previous_timestamp);
        self.latest_block = block;
    }
}
impl Default for Dynamic {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Debug, Clone)]
pub struct Trusted {
    hashes: types::Hashes,
    stakers: types::Stakers,
    balance: types::Balance,
    balance_staked: types::Balance,
}
impl Trusted {
    pub fn new() -> Self {
        Self {
            hashes: vec![],
            stakers: VecDeque::new(),
            balance: HashMap::new(),
            balance_staked: HashMap::new(),
        }
    }
    pub fn get_stakers(&self) -> &types::Stakers {
        &self.stakers
    }
    pub fn get_hashes(&self) -> &types::Hashes {
        &self.hashes
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
    fn set_stakers(&mut self, block: &Block) {
        if self.stakers.len() > 1 {
            self.stakers.rotate_left(1);
        }
        for stake in block.stakes.iter() {
            let balance_staked = self.get_balance_staked(&stake.public_key);
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
                    address::encode(&stake.public_key),
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
                debug!(
                    "{} {} {} {}",
                    "Minted".cyan(),
                    MIN_STAKE.to_string().yellow(),
                    address::encode(&block.public_key).green(),
                    hex::encode(block.hash())
                );
            }
        }
        self.set_balance(block.public_key, balance);
    }
    pub fn set_penalty(
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
    fn set(&mut self, block: &Block, previous_timestamp: types::Timestamp) {
        self.set_penalty(&block.timestamp, &previous_timestamp);
        self.set_reward(block);
        self.set_balances(block);
        self.set_stakers(block);
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hashes: &Vec<types::Hash>) {
        if hashes.is_empty() {
            return;
        }
        let mut previous_timestamp = match hashes.first() {
            Some(hash) => Block::get(db, hash).unwrap().timestamp,
            None => 0,
        };
        for hash in hashes.iter() {
            let block = Block::get(db, hash).unwrap();
            let t = block.timestamp;
            self.append(block, previous_timestamp);
            previous_timestamp = t;
        }
    }
    pub fn append(&mut self, block: Block, previous_timestamp: types::Timestamp) {
        self.hashes.push(block.hash());
        self.set(&block, previous_timestamp);
    }
}
impl Default for Trusted {
    fn default() -> Self {
        Self::new()
    }
}
