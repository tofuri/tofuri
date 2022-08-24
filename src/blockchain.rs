use crate::{
    block::{Block, BlockMetadata},
    constants::{
        BLOCK_STAKES_LIMIT, BLOCK_TIME_MAX, BLOCK_TIME_MIN, BLOCK_TRANSACTIONS_LIMIT,
        DECIMAL_PRECISION, GENESIS_TIMESTAMP, PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT,
    },
    db,
    stake::Stake,
    transaction::Transaction,
    util, wallet,
};
use colored::*;
use ed25519_dalek::Keypair;
use log::info;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, error::Error};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stakers {
    pub queue: VecDeque<([u8; 32], u64, usize)>,
}
impl Stakers {
    pub fn new(
        db: &DBWithThreadMode<SingleThreaded>,
        hashes: &[[u8; 32]],
    ) -> Result<Stakers, Box<dyn Error>> {
        let mut queue = VecDeque::new();
        for (index, hash) in hashes.iter().enumerate() {
            let block = Block::get(db, hash)?;
            for stake in block.stakes {
                queue.push_back((stake.public_key, stake.amount, index));
            }
        }
        Ok(Stakers { queue })
    }
}
#[derive(Debug)]
pub struct Blockchain {
    pub latest_block: Block,
    pub hashes: Vec<[u8; 32]>,
    pub stakers: Stakers,
    pub pending_transactions: Vec<Transaction>,
    pub pending_stakes: Vec<Stake>,
    pub pending_blocks: Vec<Block>,
}
impl Blockchain {
    pub fn new(db: &DBWithThreadMode<SingleThreaded>) -> Result<Blockchain, Box<dyn Error>> {
        let latest_block = Blockchain::get_latest_block(db)?;
        let hashes = Blockchain::hashes(db, BlockMetadata::from(&latest_block).hash())?;
        // reinitialize latest_block in case of a broken chain
        let latest_block = Blockchain::get_latest_block(db)?;
        let validators = Stakers::new(db, &hashes)?;
        Ok(Blockchain {
            latest_block,
            hashes,
            stakers: validators,
            pending_transactions: vec![],
            pending_stakes: vec![],
            pending_blocks: vec![],
        })
    }
    pub fn forge_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        keypair: &Keypair,
    ) -> Result<Block, Box<dyn Error>> {
        let latest_block_metadata = BlockMetadata::from(&self.latest_block);
        let mut block = Block::new(latest_block_metadata.hash());
        self.sort_pending_transactions();
        let pending_transactions = self.pending_transactions.clone();
        self.pending_transactions.clear();
        for transaction in pending_transactions {
            if block.transactions.len() < BLOCK_TRANSACTIONS_LIMIT {
                block.transactions.push(transaction);
            } else {
                self.pending_transactions.push(transaction);
            }
        }
        self.sort_pending_stakes();
        let pending_stakes = self.pending_stakes.clone();
        self.pending_stakes.clear();
        for stake in pending_stakes {
            if block.stakes.len() < BLOCK_STAKES_LIMIT {
                block.stakes.push(stake);
            } else {
                self.pending_stakes.push(stake);
            }
        }
        let mut block_metadata = BlockMetadata::from(&block);
        block_metadata.sign(keypair);
        block.public_key = block_metadata.public_key;
        block.signature = block_metadata.signature;
        self.try_add_block(db, block.clone())?;
        info!(
            "{}: {} @ {}",
            "Forged".cyan(),
            (self.latest_height() + 1).to_string().yellow(),
            hex::encode(block_metadata.hash()).green()
        );
        Ok(block)
    }
    pub fn try_add_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        block: Block,
    ) -> Result<(), Box<dyn Error>> {
        // check if block is valid
        if !block.is_valid() {
            return Err("block is not valid".into());
        }
        // check if previous block exists
        let previous_block = Block::get(db, &block.previous_hash)?;
        // check if block extends active chain
        if self.latest_block.previous_hash != previous_block.previous_hash {
            return Err("block does not extend active chain".into());
        }
        self.pending_blocks.push(block);
        // check validator
        // self.validate_block(db, block);
        Ok(())
    }
    pub fn try_add_transaction(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        transaction: Transaction,
    ) -> Result<(), Box<dyn Error>> {
        // check if transaction is valid
        if !transaction.is_valid() {
            return Err("transaction is not valid".into());
        }
        // check if transaction is already pending
        if self
            .pending_transactions
            .iter()
            .any(|x| x.signature == transaction.signature)
        {
            return Err("transaction is already pending".into());
        }
        // check if transaction is already included in chain (i.e. not a new transaction)
        if Transaction::get(db, &transaction.hash()).is_ok() {
            return Err("transaction is already included in chain".into());
        }
        // check if input affords sum
        let mut transactions = vec![transaction.clone()];
        for _ in 0..self.pending_transactions.len() {
            for (index, t) in self.pending_transactions.iter().enumerate() {
                if t.input == transaction.input {
                    transactions.push(self.pending_transactions.swap_remove(index));
                    break;
                }
            }
        }
        transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        let balance = self.get_balance(db, &transaction.input)?;
        let mut sum = 0;
        for t in transactions {
            if sum + t.amount + t.fee <= balance {
                sum += t.amount + t.fee;
                self.pending_transactions.push(t);
            } else {
                break;
            }
        }
        // if transaction.timestamp < self.latest_block.timestamp {
        //     return Err("transaction old".into());
        // }
        self.sort_pending_transactions();
        self.limit_pending_transactions();
        Ok(())
    }
    pub fn try_add_stake(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        stake: Stake,
    ) -> Result<(), Box<dyn Error>> {
        // check if transaction is valid
        if !stake.is_valid() {
            return Err("stake is not valid".into());
        }
        // check if stake is already pending
        if self
            .pending_stakes
            .iter()
            .any(|x| x.signature == stake.signature)
        {
            return Err("stake is already pending".into());
        }
        // check if stake is already included in chain (i.e. not a new stake)
        if Stake::get(db, &stake.hash()).is_ok() {
            return Err("stake is already included in chain".into());
        }
        // check if input affords sum
        let mut stakes = vec![stake.clone()];
        for _ in 0..self.pending_stakes.len() {
            for (index, t) in self.pending_stakes.iter().enumerate() {
                if t.public_key == stake.public_key {
                    stakes.push(self.pending_stakes.swap_remove(index));
                    break;
                }
            }
        }
        stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
        let balance = self.get_balance(db, &stake.public_key)?;
        let mut sum = 0;
        for t in stakes {
            if sum + t.amount + t.fee <= balance {
                sum += t.amount + t.fee;
                self.pending_stakes.push(t);
            } else {
                break;
            }
        }
        if stake.timestamp < self.latest_block.timestamp {
            return Err("stake old".into());
        }
        self.sort_pending_stakes();
        self.limit_pending_stakes();
        Ok(())
    }
    pub fn height(&self, hash: [u8; 32]) -> Option<usize> {
        self.hashes.iter().position(|&x| x == hash)
    }
    pub fn latest_height(&self) -> usize {
        self.hashes.len() - 1
    }
    fn hashes(
        db: &DBWithThreadMode<SingleThreaded>,
        previous_hash: [u8; 32],
    ) -> Result<Vec<[u8; 32]>, Box<dyn Error>> {
        let mut hashes = vec![];
        let mut previous_hash = previous_hash;
        let mut closure = || -> Result<Option<()>, Box<dyn Error>> {
            match Block::get(db, &previous_hash) {
                Ok(block) => {
                    let block_metadata = BlockMetadata::from(&block);
                    let hash = block_metadata.hash();
                    if hash != previous_hash {
                        log::error!(
                            "{}: {} != {}",
                            "Detected broken chain!".red(),
                            hex::encode(hash),
                            hex::encode(previous_hash)
                        );
                        log::warn!("{}", "Pruning broken chain".yellow());
                        hashes.clear();
                        Blockchain::put_latest_block_hash(db, previous_hash)?;
                    }
                    hashes.push(hash);
                    previous_hash = block.previous_hash;
                    Ok(Some(()))
                }
                Err(err) => {
                    if err.to_string() == "block not found" {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                }
            }
        };
        while (closure()?).is_some() {}
        Ok(hashes)
    }
    fn genesis_block() -> Block {
        Block::from([0; 32], GENESIS_TIMESTAMP, [0; 32], [0; 64])
    }
    fn get_latest_block(db: &DBWithThreadMode<SingleThreaded>) -> Result<Block, Box<dyn Error>> {
        let bytes = db.get(db::key(&db::Key::LatestBlockHash))?;
        if let Some(bytes) = bytes {
            // latest_block is set
            Block::get(db, &bytes)
        } else {
            // latest_block is NOT set
            // should be the case if the blockchain haven't been initialized
            let block = Blockchain::genesis_block();
            block.put(db)?;
            let block_metadata = BlockMetadata::from(&block);
            Blockchain::put_latest_block_hash(db, block_metadata.hash())?;
            Ok(block)
        }
    }
    fn put_latest_block_hash(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: [u8; 32],
    ) -> Result<(), Box<dyn Error>> {
        db.put(db::key(&db::Key::LatestBlockHash), hash)?;
        Ok(())
    }
    fn get_balance_raw(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &[u8; 32],
    ) -> Result<u64, Box<dyn Error>> {
        let bytes = db
            .get_cf(db::cf_handle_balances(db)?, public_key)?
            .ok_or("balance not found")?;
        Ok(u64::from_le_bytes(bytes.as_slice().try_into()?))
    }
    pub fn get_balance(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &[u8; 32],
    ) -> Result<u64, Box<dyn Error>> {
        match self.get_balance_raw(db, public_key) {
            Ok(balance) => Ok(balance),
            Err(err) => {
                if err.to_string() == "balance not found" {
                    Ok(0)
                } else {
                    Err(err)
                }
            }
        }
    }
    fn put_balance(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &[u8; 32],
        balance: u64,
    ) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::cf_handle_balances(db)?,
            public_key,
            balance.to_le_bytes(),
        )?;
        Ok(())
    }
    fn get_staked_balance_raw(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &[u8; 32],
    ) -> Result<u64, Box<dyn Error>> {
        let bytes = db
            .get_cf(db::cf_handle_staked_balances(db)?, public_key)?
            .ok_or("staked_balance not found")?;
        Ok(u64::from_le_bytes(bytes.as_slice().try_into()?))
    }
    pub fn get_staked_balance(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &[u8; 32],
    ) -> Result<u64, Box<dyn Error>> {
        match self.get_staked_balance_raw(db, public_key) {
            Ok(balance) => Ok(balance),
            Err(err) => {
                if err.to_string() == "staked_balance not found" {
                    Ok(0)
                } else {
                    Err(err)
                }
            }
        }
    }
    fn put_staked_balance(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &[u8; 32],
        staked_balance: u64,
    ) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::cf_handle_staked_balances(db)?,
            public_key,
            staked_balance.to_le_bytes(),
        )?;
        Ok(())
    }
    fn add_reward(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &[u8; 32],
        stake_amount: u64,
        fees: u64,
    ) -> Result<(), Box<dyn Error>> {
        let mut balance = self.get_balance(db, public_key)?;
        balance += Blockchain::reward(stake_amount);
        balance += fees;
        self.put_balance(db, public_key, balance)?;
        Ok(())
    }
    fn cache_balances(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        transactions: &Vec<Transaction>,
        stakes: &Vec<Stake>,
    ) -> Result<(), Box<dyn Error>> {
        for t in transactions {
            // input
            let mut balance = self.get_balance(db, &t.input)?;
            balance -= t.amount + t.fee;
            self.put_balance(db, &t.input, balance)?;
            // output
            let mut balance = self.get_balance(db, &t.output)?;
            balance += t.amount;
            self.put_balance(db, &t.output, balance)?;
        }
        for s in stakes {
            if s.deposit {
                // input
                let mut balance = self.get_balance(db, &s.public_key)?;
                balance -= s.amount + s.fee;
                self.put_balance(db, &s.public_key, balance)?;
                let mut staked_balance = self.get_staked_balance(db, &s.public_key)?;
                staked_balance += s.amount;
                self.put_staked_balance(db, &s.public_key, staked_balance)?;
            } else {
                // output
                let mut balance = self.get_balance(db, &s.public_key)?;
                balance += s.amount;
                balance -= s.fee;
                self.put_balance(db, &s.public_key, balance)?;
                let mut staked_balance = self.get_staked_balance(db, &s.public_key)?;
                staked_balance -= s.amount;
                self.put_staked_balance(db, &s.public_key, staked_balance)?;
            }
        }
        Ok(())
    }
    fn get_fees(transactions: &Vec<Transaction>, stakes: &Vec<Stake>) -> u64 {
        let mut fees = 0;
        for t in transactions {
            fees += t.fee;
        }
        for s in stakes {
            fees += s.fee;
        }
        fees
    }
    fn sort_pending_transactions(&mut self) {
        self.pending_transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
    }
    fn limit_pending_transactions(&mut self) {
        while self.pending_transactions.len() > PENDING_TRANSACTIONS_LIMIT {
            self.pending_transactions
                .remove(self.pending_transactions.len() - 1);
        }
    }
    fn sort_pending_stakes(&mut self) {
        self.pending_stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
    }
    fn limit_pending_stakes(&mut self) {
        while self.pending_stakes.len() > PENDING_STAKES_LIMIT {
            self.pending_stakes.remove(self.pending_stakes.len() - 1);
        }
    }
    pub fn reward(stake_amount: u64) -> u64 {
        ((2f64.powf((stake_amount as f64 / DECIMAL_PRECISION as f64) / 100f64) - 1f64)
            * DECIMAL_PRECISION as f64) as u64
    }
    pub fn accept_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        forger: bool,
    ) -> Result<(), Box<dyn Error>> {
        if self.pending_blocks.is_empty() {
            if !self.stakers.queue.is_empty()
                && util::timestamp() > self.latest_block.timestamp + BLOCK_TIME_MAX as u64
            {
                self.punish_staker(db, self.stakers.queue[0])?;
            }
            return Err("no pending blocks".into());
        }
        let block;
        if self.stakers.queue.is_empty() {
            block = self.pending_blocks.remove(0);
            self.cold_start_mint_stakers_stakes(db, &block)?;
        } else {
            match self
                .pending_blocks
                .iter()
                .position(|x| x.public_key == self.stakers.queue[0].0)
            {
                Some(index) => block = self.pending_blocks.remove(index),
                None => {
                    self.punish_staker(db, self.stakers.queue[0])?;
                    return Err("validator did not show up".into());
                }
            }
        }
        if !self.stakers.queue.is_empty()
            && (block.timestamp < self.latest_block.timestamp + BLOCK_TIME_MIN as u64
                || block.timestamp > self.latest_block.timestamp + BLOCK_TIME_MAX as u64)
        {
            self.punish_staker(db, self.stakers.queue[0])?;
            return Err("validator did not show up in time".into());
        }
        // save block
        block.put(db)?;
        let block_metadata = BlockMetadata::from(&block);
        let hash = block_metadata.hash();
        self.hashes.push(hash);
        self.cache_balances(db, &block.transactions, &block.stakes)?;
        // set latest block
        Blockchain::put_latest_block_hash(db, hash)?;
        self.latest_block = block;
        // append new validators to queue
        for stake in self.latest_block.stakes.iter() {
            self.stakers
                .queue
                .push_back((stake.public_key, stake.amount, self.latest_height()));
        }
        // reward validator
        let stake_amount = self.stakers.queue[0].1;
        let fees = Blockchain::get_fees(&self.latest_block.transactions, &self.latest_block.stakes);
        self.add_reward(db, &self.latest_block.public_key, stake_amount, fees)?;
        // rotate validator queue
        if !self.stakers.queue.is_empty() {
            self.stakers.queue.rotate_left(1);
        }
        if !forger {
            info!(
                "{}: {} {}",
                "Accepted".green(),
                self.latest_height().to_string().yellow(),
                hex::encode(hash)
            );
        }
        self.pending_blocks.clear();
        Ok(())
    }
    fn punish_staker(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        staker: ([u8; 32], u64, usize),
    ) -> Result<(), Box<dyn Error>> {
        let public_key = staker.0;
        let amount = staker.1;
        let mut staked_balance = self.get_staked_balance(db, &public_key)?;
        if staked_balance != 0 {
            // maybe bugfix (thread 'main' panicked at 'attempt to subtract with overflow')
            staked_balance -= amount;
        }
        self.put_staked_balance(db, &public_key, staked_balance)?;
        self.stakers.queue.remove(0).unwrap();
        log::warn!(
            "{}: {} {}",
            "Burned".red(),
            amount.to_string().yellow(),
            wallet::address::encode(&public_key)
        );
        Ok(())
    }
    fn cold_start_mint_stakers_stakes(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        block: &Block,
    ) -> Result<(), Box<dyn Error>> {
        log::warn!(
            "{}",
            "Staker queue should not be empty unless the network just started up.".yellow()
        );
        for stake in block.stakes.iter() {
            let mut balance = self.get_balance(db, &stake.public_key)?;
            let minted = stake.amount + stake.fee;
            balance += minted;
            self.put_balance(db, &stake.public_key, balance)?;
            log::warn!(
                "{}: {} @ {}",
                "Minted".cyan(),
                minted.to_string().yellow(),
                wallet::address::encode(&stake.public_key).green()
            );
        }
        Ok(())
    }
}
