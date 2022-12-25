use crate::{state::Dynamic, states::States, sync::Sync};
use colored::*;
use log::{debug, info, warn};
use pea_block::Block;
use pea_core::constants::{
    BLOCK_STAKES_LIMIT, BLOCK_TIME_MIN, BLOCK_TRANSACTIONS_LIMIT, PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT, STAKE, SYNC_BLOCKS_PER_TICK,
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
    pub offline: HashMap<types::AddressBytes, types::Hash>,
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
        self.sync.index = self.height().saturating_sub(self.trust_fork_after_blocks + SYNC_BLOCKS_PER_TICK);
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
    pub fn forge_block(&mut self, timestamp: u32) -> Option<Block> {
        if let Some(address) = self.states.dynamic.current_staker(timestamp) {
            if address != &self.key.address_bytes() || timestamp < self.states.dynamic.latest_block.timestamp + BLOCK_TIME_MIN as u32 {
                return None;
            }
        } else {
            let mut stake = Stake::new(true, 0, timestamp);
            stake.sign(&self.key);
            self.pending_stakes = vec![stake];
        }
        let mut block = if let Some(main) = self.tree.main() {
            Block::new(main.0, timestamp)
        } else {
            Block::new([0; 32], timestamp)
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
        let info_0 = if forged { "Forged".magenta() } else { "Accept".green() };
        let info_1 = hex::encode(hash);
        if let Some(main) = self.tree.main() {
            if hash == main.0 {
                self.pending_transactions.clear();
                self.pending_stakes.clear();
                if !forged {
                    self.sync.new += 1;
                }
                info!("{} {} {}", info_0, main.1.to_string().yellow(), info_1);
                return;
            }
        }
        info!("{} {}", info_0, info_1);
    }
    pub fn try_add_transaction(&mut self, transaction: Transaction, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if let Some(index) = self
            .pending_transactions
            .iter()
            .position(|s| s.input_public_key == transaction.input_public_key)
        {
            if transaction.fee <= self.pending_transactions[index].fee {
                return Err("transaction fee too low".into());
            }
            self.pending_transactions.remove(index);
        }
        self.validate_transaction(&transaction, self.states.dynamic.latest_block.timestamp, timestamp)?;
        info!("Transaction {}", hex::encode(&transaction.hash()).green());
        self.pending_transactions.push(transaction);
        self.pending_transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        while self.pending_transactions.len() > PENDING_TRANSACTIONS_LIMIT {
            self.pending_transactions.remove(self.pending_transactions.len() - 1);
        }
        Ok(())
    }
    pub fn try_add_stake(&mut self, stake: Stake, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if let Some(index) = self.pending_stakes.iter().position(|s| s.public_key == stake.public_key) {
            if stake.fee <= self.pending_stakes[index].fee {
                return Err("stake fee too low".into());
            }
            self.pending_stakes.remove(index);
        }
        self.validate_stake(&stake, self.states.dynamic.latest_block.timestamp, timestamp)?;
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
    pub fn validate_block(&self, block: &Block, timestamp: u32) -> Result<(), Box<dyn Error>> {
        let address = util::address(&block.public_key);
        if let Some(hash) = self.offline.get(&address) {
            if hash == &block.previous_hash {
                return Err("block staker banned".into());
            }
        }
        if self.tree.get(&block.hash()).is_some() {
            return Err("block hash in tree".into());
        }
        if block.timestamp > timestamp + self.time_delta {
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
        if let Some(a) = dynamic.staker(block.timestamp, latest_block.timestamp) {
            if a != &address {
                return Err("block staker address".into());
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
            self.validate_stake(stake, latest_block.timestamp, timestamp)?;
        }
        for transaction in block.transactions.iter() {
            self.validate_transaction(transaction, latest_block.timestamp, timestamp)?;
        }
        Ok(())
    }
    fn validate_transaction(&self, transaction: &Transaction, previous_block_timestamp: u32, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if self.pending_transactions.iter().any(|x| x.signature == transaction.signature) {
            return Err("transaction pending".into());
        }
        transaction.validate()?;
        let balance = self.states.dynamic.balance(&util::address(&transaction.input_public_key));
        if transaction.timestamp > timestamp + self.time_delta {
            return Err("transaction timestamp future".into());
        }
        if transaction.timestamp < previous_block_timestamp {
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
    fn validate_stake(&self, stake: &Stake, previous_block_timestamp: u32, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if self.pending_stakes.iter().any(|x| x.signature == stake.signature) {
            return Err("stake pending".into());
        }
        stake.validate()?;
        let address = util::address(&stake.public_key);
        let balance = self.states.dynamic.balance(&address);
        let balance_staked = self.states.dynamic.balance_staked(&address);
        if stake.timestamp > timestamp + self.time_delta {
            return Err("stake timestamp future".into());
        }
        if stake.timestamp < previous_block_timestamp {
            return Err("stake timestamp ancient".into());
        }
        if stake.deposit {
            if STAKE + stake.fee > balance {
                return Err("stake deposit too expensive".into());
            }
            if balance_staked != 0 {
                return Err("stake already staking".into());
            }
        } else {
            if stake.fee > balance {
                return Err("stake withdraw fee too expensive".into());
            }
            if STAKE > balance_staked {
                return Err("stake withdraw too expensive".into());
            }
        }
        if db::stake::get(&self.db, &stake.hash()).is_ok() {
            return Err("stake in chain".into());
        }
        Ok(())
    }
}
