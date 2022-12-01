use crate::{state::Dynamic, states::States, sync::Sync};
use colored::*;
use log::{debug, info, warn};
use pea_block::Block;
use pea_core::constants::{
    BLOCK_STAKES_LIMIT, BLOCK_TIME_MIN, BLOCK_TRANSACTIONS_LIMIT, MAX_STAKE, MIN_STAKE, PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT,
};
use pea_core::{types, util};
use pea_db as db;
use pea_key::Key;
use pea_stake::Stake;
use pea_transaction::Transaction;
use pea_tree::Tree;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::collections::HashMap;
use std::{error::Error, time::Instant};
#[derive(Debug)]
pub struct Blockchain {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub key: Key,
    pub tree: Tree,
    pub states: States,
    pub pending_transactions: Vec<Transaction>,
    pub pending_stakes: Vec<Stake>,
    pub pending_blocks: Vec<Block>,
    pub sync: Sync,
    pub trust_fork_after_blocks: usize,
    pub pending_blocks_limit: usize,
    pub time_delta: u32,
    pub offline: HashMap<types::PublicKeyBytes, types::Hash>,
}
impl Blockchain {
    pub fn new(db: DBWithThreadMode<SingleThreaded>, key: Key, trust_fork_after_blocks: usize, pending_blocks_limit: usize, time_delta: u32) -> Self {
        Self {
            db,
            key,
            tree: Tree::default(),
            states: States::default(),
            pending_transactions: vec![],
            pending_stakes: vec![],
            pending_blocks: vec![],
            sync: Sync::default(),
            trust_fork_after_blocks,
            pending_blocks_limit,
            time_delta,
            offline: HashMap::new(),
        }
    }
    pub fn load(&mut self) {
        let start = Instant::now();
        db::tree::reload(&mut self.tree, &self.db);
        info!("Loaded tree in {}", format!("{:?}", start.elapsed()).yellow());
        let start = Instant::now();
        let (hashes_trusted, hashes_dynamic) = self.tree.hashes(self.trust_fork_after_blocks);
        self.states.trusted.load(&self.db, &hashes_trusted);
        self.states.dynamic = Dynamic::from(&self.db, &hashes_dynamic, &self.states.trusted);
        info!("Loaded states in {}", format!("{:?}", start.elapsed()).yellow());
    }
    pub fn height(&self) -> usize {
        if let Some(main) = self.tree.main() {
            main.1
        } else {
            0
        }
    }
    pub fn forge_block(&mut self) -> Option<Block> {
        let timestamp = util::timestamp();
        if let Some(public_key) = self.states.dynamic.current_staker() {
            if public_key != &self.key.public_key_bytes() || timestamp < self.states.dynamic.latest_block.timestamp + BLOCK_TIME_MIN as u32 {
                return None;
            }
        } else {
            let mut stake = Stake::new(true, MIN_STAKE, 0);
            stake.sign(&self.key);
            self.pending_stakes = vec![stake];
        }
        let mut block = if let Some(main) = self.tree.main() {
            Block::new(main.0)
        } else {
            Block::new([0; 32])
        };
        for transaction in self.pending_transactions.iter() {
            if block.transactions.len() < BLOCK_TRANSACTIONS_LIMIT {
                block.transactions.push(transaction.clone());
            }
        }
        for stake in self.pending_stakes.iter() {
            if block.stakes.len() < BLOCK_STAKES_LIMIT {
                block.stakes.push(stake.clone());
            }
        }
        block.sign(&self.key);
        self.accept_block(&block, true);
        Some(block)
    }
    pub fn accept_block(&mut self, block: &Block, forged: bool) {
        db::block::put(block, &self.db).unwrap();
        let hash = block.hash();
        if self.tree.insert(hash, block.previous_hash, block.timestamp).unwrap() {
            warn!("{} {}", "Forked".red(), hex::encode(hash));
        }
        self.tree.sort_branches();
        self.states
            .update(&self.db, &self.tree.hashes_dynamic(self.trust_fork_after_blocks), self.trust_fork_after_blocks);
        if let Some(index) = self.pending_blocks.iter().position(|x| x.hash() == hash) {
            self.pending_blocks.remove(index);
        }
        self.pending_transactions.clear();
        self.pending_stakes.clear();
        if !forged && block.hash() == self.states.dynamic.latest_block.hash() {
            self.sync.new += 1;
        }
        info!(
            "{} {} {}",
            if forged { "Forged".magenta() } else { "Accept".green() },
            self.tree.height(&block.previous_hash).to_string().yellow(),
            hex::encode(hash)
        );
    }
    pub fn accept_pending_blocks(&mut self) {
        for block in self.pending_blocks.clone() {
            self.accept_block(&block, false);
        }
    }
    pub fn try_add_block(&mut self, block: Block) -> Result<(), Box<dyn Error>> {
        if self.pending_blocks.iter().any(|b| b.signature == block.signature) {
            return Err("block pending".into());
        }
        self.validate_block(&block)?;
        self.pending_blocks.push(block);
        self.pending_blocks.sort_by(|a, b| b.fees().cmp(&a.fees()));
        while self.pending_blocks.len() > self.pending_blocks_limit {
            self.pending_blocks.remove(self.pending_blocks.len() - 1);
        }
        Ok(())
    }
    pub fn try_add_transaction(&mut self, transaction: Transaction) -> Result<(), Box<dyn Error>> {
        if let Some(index) = self
            .pending_transactions
            .iter()
            .position(|s| s.public_key_input == transaction.public_key_input)
        {
            if transaction.fee <= self.pending_transactions[index].fee {
                return Err("transaction fee too low".into());
            }
            self.pending_transactions.remove(index);
        }
        self.validate_transaction(&transaction, self.states.dynamic.latest_block.timestamp)?;
        info!("Transaction {}", hex::encode(&transaction.hash()).green());
        self.pending_transactions.push(transaction);
        self.pending_transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        while self.pending_transactions.len() > PENDING_TRANSACTIONS_LIMIT {
            self.pending_transactions.remove(self.pending_transactions.len() - 1);
        }
        Ok(())
    }
    pub fn try_add_stake(&mut self, stake: Stake) -> Result<(), Box<dyn Error>> {
        if let Some(index) = self.pending_stakes.iter().position(|s| s.public_key == stake.public_key) {
            if stake.fee <= self.pending_stakes[index].fee {
                return Err("stake fee too low".into());
            }
            self.pending_stakes.remove(index);
        }
        self.validate_stake(&stake, self.states.dynamic.latest_block.timestamp)?;
        info!("Stake {}", hex::encode(&stake.hash()).green());
        self.pending_stakes.push(stake);
        self.pending_stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
        while self.pending_stakes.len() > PENDING_STAKES_LIMIT {
            self.pending_stakes.remove(self.pending_stakes.len() - 1);
        }
        Ok(())
    }
    pub fn sync_block(&mut self) -> Block {
        let hashes_trusted = &self.states.trusted.hashes;
        let hashes_dynamic = &self.states.dynamic.hashes;
        if self.sync.index >= hashes_trusted.len() + hashes_dynamic.len() {
            self.sync.index = 0;
        }
        let hash = if self.sync.index < hashes_trusted.len() {
            hashes_trusted[self.sync.index]
        } else {
            hashes_dynamic[self.sync.index - hashes_trusted.len()]
        };
        debug!("{} {} {}", "Sync".cyan(), self.sync.index.to_string().yellow(), hex::encode(hash));
        let block = db::block::get(&self.db, &hash).unwrap();
        self.sync.index += 1;
        block
    }
    fn validate_block(&self, block: &Block) -> Result<(), Box<dyn Error>> {
        if let Some(hash) = self.offline.get(&block.public_key) {
            if hash == &block.previous_hash {
                return Err("block staker banned".into());
            }
        }
        if self.tree.get(&block.hash()).is_some() {
            return Err("block hash in tree".into());
        }
        if block.timestamp > util::timestamp() + self.time_delta {
            return Err("block timestamp future".into());
        }
        if block.previous_hash != [0; 32] && self.tree.get(&block.previous_hash).is_none() {
            return Err("block previous_hash not in tree".into());
        }
        let dynamic = self.states.dynamic_fork(self, &block.previous_hash)?;
        let latest_block = &dynamic.latest_block;
        if block.timestamp < latest_block.timestamp + BLOCK_TIME_MIN as u32 {
            return Err("block timestamp early".into());
        }
        if let Some(public_key) = dynamic.staker(block.timestamp, latest_block.timestamp) {
            if public_key != &block.public_key {
                return Err("block staker public_key".into());
            }
        } else {
            block.validate_mint()?;
            return Ok(());
        }
        block.validate()?;
        if block.previous_hash != latest_block.hash() {
            return Err("block previous_hash not latest hash".into());
        }
        for stake in block.stakes.iter() {
            self.validate_stake(stake, latest_block.timestamp)?;
        }
        for transaction in block.transactions.iter() {
            self.validate_transaction(transaction, latest_block.timestamp)?;
        }
        Ok(())
    }
    fn validate_transaction(&self, transaction: &Transaction, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if self.pending_transactions.iter().any(|x| x.signature == transaction.signature) {
            return Err("transaction pending".into());
        }
        transaction.validate()?;
        let balance = self.states.dynamic.balance(&transaction.public_key_input);
        if transaction.timestamp > util::timestamp() + self.time_delta {
            return Err("transaction timestamp future".into());
        }
        if transaction.timestamp < timestamp {
            return Err("transaction timestamp ancient".into());
        }
        if transaction.amount + transaction.fee > balance {
            return Err("transaction too expensive".into());
        }
        if db::transaction::get(&self.db, &transaction.hash()).is_ok() {
            return Err("transaction in chain".into());
        }
        Ok(())
    }
    fn validate_stake(&self, stake: &Stake, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if self.pending_stakes.iter().any(|x| x.signature == stake.signature) {
            return Err("stake pending".into());
        }
        stake.validate()?;
        let balance = self.states.dynamic.balance(&stake.public_key);
        let balance_staked = self.states.dynamic.balance_staked(&stake.public_key);
        if stake.timestamp > util::timestamp() + self.time_delta {
            return Err("stake timestamp future".into());
        }
        if stake.timestamp < timestamp {
            return Err("stake timestamp ancient".into());
        }
        if stake.deposit {
            if stake.amount + stake.fee > balance {
                return Err("stake deposit too expensive".into());
            }
            if stake.amount + balance_staked > MAX_STAKE {
                return Err("stake deposit exceeds MAX_STAKE".into());
            }
        } else {
            if stake.fee > balance {
                return Err("stake withdraw fee too expensive".into());
            }
            if stake.amount > balance_staked {
                return Err("stake withdraw too expensive".into());
            }
        }
        if db::stake::get(&self.db, &stake.hash()).is_ok() {
            return Err("stake in chain".into());
        }
        Ok(())
    }
}
