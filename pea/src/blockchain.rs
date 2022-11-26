use crate::{state::Dynamic, states::States, sync::Sync};
use colored::*;
use log::{debug, info, warn};
use pea_block::Block;
use pea_core::util;
use pea_core::{
    constants::{BLOCK_STAKES_LIMIT, BLOCK_TIME_MIN, BLOCK_TRANSACTIONS_LIMIT, MAX_STAKE, MIN_STAKE, PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT},
    types,
};
use pea_db as db;
use pea_key::Key;
use pea_stake::Stake;
use pea_transaction::Transaction;
use pea_tree::Tree;
use rocksdb::{DBWithThreadMode, SingleThreaded};
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
}
impl Blockchain {
    pub fn new(db: DBWithThreadMode<SingleThreaded>, key: Key, trust_fork_after_blocks: usize, pending_blocks_limit: usize) -> Self {
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
        if let Some(public_key) = self.states.dynamic.staker(timestamp, self.states.dynamic.latest_block.timestamp) {
            if public_key != &self.key.public_key_bytes() || timestamp < self.states.dynamic.latest_block.timestamp + BLOCK_TIME_MIN as u32 {
                return None;
            }
        } else {
            let mut stake = Stake::new(true, MIN_STAKE, 0).unwrap();
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
        if block.hash() == self.states.dynamic.latest_block.hash() {
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
            return Err("block already pending".into());
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
        if self.pending_transactions.iter().any(|x| x.signature == transaction.signature) {
            return Err("transaction already pending".into());
        }
        if let Some(index) = self
            .pending_transactions
            .iter()
            .position(|s| s.public_key_input == transaction.public_key_input)
        {
            if transaction.fee <= self.pending_transactions[index].fee {
                return Err("transaction fee too low to replace previous pending transaction".into());
            }
            self.pending_transactions.remove(index);
        }
        let balance = self.states.dynamic.balance(&transaction.public_key_input);
        self.validate_transaction(&transaction, balance, self.states.dynamic.latest_block.timestamp)?;
        self.pending_transactions.push(transaction);
        self.pending_transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        while self.pending_transactions.len() > PENDING_TRANSACTIONS_LIMIT {
            self.pending_transactions.remove(self.pending_transactions.len() - 1);
        }
        Ok(())
    }
    pub fn try_add_stake(&mut self, stake: Stake) -> Result<(), Box<dyn Error>> {
        if self.pending_stakes.iter().any(|x| x.signature == stake.signature) {
            return Err("stake already pending".into());
        }
        if let Some(index) = self.pending_stakes.iter().position(|s| s.public_key == stake.public_key) {
            if stake.fee <= self.pending_stakes[index].fee {
                return Err("stake fee too low to replace previous pending stake".into());
            }
            self.pending_stakes.remove(index);
        }
        let balance = self.states.dynamic.balance(&stake.public_key);
        let balance_staked = self.states.dynamic.balance_staked(&stake.public_key);
        self.validate_stake(&stake, balance, balance_staked, self.states.dynamic.latest_block.timestamp)?;
        self.pending_stakes.push(stake);
        self.pending_stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
        while self.pending_stakes.len() > PENDING_STAKES_LIMIT {
            self.pending_stakes.remove(self.pending_stakes.len() - 1);
        }
        Ok(())
    }
    pub fn sync_blocks(&mut self) -> [Block; 2] {
        [self.sync_block_0(), self.sync_block_1()]
    }
    fn sync_block_0(&mut self) -> Block {
        let hashes_trusted = &self.states.trusted.hashes;
        let hashes_dynamic = &self.states.dynamic.hashes;
        if self.sync.index_0 >= hashes_trusted.len() + hashes_dynamic.len() {
            self.sync.index_0 = 0;
        }
        let hash = if self.sync.index_0 < hashes_trusted.len() {
            hashes_trusted[self.sync.index_0]
        } else {
            hashes_dynamic[self.sync.index_0 - hashes_trusted.len()]
        };
        debug!("{} {} {}", "Sync 0".cyan(), self.sync.index_0.to_string().yellow(), hex::encode(hash));
        let block = db::block::get(&self.db, &hash).unwrap();
        self.sync.index_0 += 1;
        block
    }
    fn sync_block_1(&mut self) -> Block {
        let hashes_dynamic = &self.states.dynamic.hashes;
        if self.sync.index_1 >= hashes_dynamic.len() {
            self.sync.index_1 = 0;
        }
        let hash = hashes_dynamic[self.sync.index_1];
        debug!("{} {} {}", "Sync 1".cyan(), self.sync.index_1.to_string().yellow(), hex::encode(hash));
        let block = db::block::get(&self.db, &hash).unwrap();
        self.sync.index_1 += 1;
        block
    }
    fn validate_block(&self, block: &Block) -> Result<(), Box<dyn Error>> {
        if block.previous_hash != [0; 32] && self.tree.get(&block.previous_hash).is_none() {
            return Err("block doesn't extend chain".into());
        }
        let dynamic = self.states.dynamic_fork(self, &block.previous_hash)?;
        let latest_block = &dynamic.latest_block;
        if block.previous_hash != [0; 32] {
            if block.previous_hash != latest_block.hash() {
                return Err("fork_state latest_block hash".into());
            }
            if let Some(public_key) = dynamic.staker(block.timestamp, latest_block.timestamp) {
                if public_key != &block.public_key {
                    return Err("block isn't signed by the staker first in queue".into());
                }
            }
        }
        if block.timestamp < latest_block.timestamp + BLOCK_TIME_MIN as u32 {
            return Err("block created too early".into());
        }
        let public_key_inputs = block.transactions.iter().map(|t| t.public_key_input).collect::<Vec<types::PublicKeyBytes>>();
        if (1..public_key_inputs.len()).any(|i| public_key_inputs[i..].contains(&public_key_inputs[i - 1])) {
            return Err("block includes multiple transactions from same public_key_input".into());
        }
        let public_keys = block.stakes.iter().map(|s| s.public_key).collect::<Vec<types::PublicKeyBytes>>();
        if (1..public_keys.len()).any(|i| public_keys[i..].contains(&public_keys[i - 1])) {
            return Err("block includes multiple stakes from same public_key".into());
        }
        if block.verify().is_err() {
            return Err("block has invalid signature".into());
        }
        if block.timestamp > util::timestamp() {
            return Err("block has invalid timestamp (block is from the future)".into());
        }
        if self.tree.get(&block.hash()).is_some() {
            return Err("block hash already in tree".into());
        }
        if !block.stakes.is_empty() {
            let stake = block.stakes.get(0).unwrap();
            if stake.fee == 0 {
                if block.stakes.len() != 1 {
                    return Err("only allowed to mint 1 stake".into());
                }
                if stake.verify().is_err() {
                    return Err("mint stake has invalid signature".into());
                }
                if stake.timestamp > util::timestamp() {
                    return Err("mint stake has invalid timestamp (mint stake is from the future)".into());
                }
                if stake.timestamp < block.timestamp {
                    return Err("mint stake too old".into());
                }
                if !stake.deposit {
                    return Err("mint stake must be deposit".into());
                }
                if stake.amount != MIN_STAKE {
                    return Err("mint stake invalid amount".into());
                }
                if stake.fee != 0 {
                    return Err("mint stake invalid fee".into());
                }
            } else {
                for stake in block.stakes.iter() {
                    let balance = dynamic.balance(&stake.public_key);
                    let balance_staked = dynamic.balance_staked(&stake.public_key);
                    self.validate_stake(stake, balance, balance_staked, latest_block.timestamp)?;
                }
            }
        }
        for transaction in block.transactions.iter() {
            let balance = dynamic.balance(&transaction.public_key_input);
            self.validate_transaction(transaction, balance, latest_block.timestamp)?;
        }
        Ok(())
    }
    fn validate_transaction(&self, transaction: &Transaction, balance: u128, timestamp: u32) -> Result<(), Box<dyn Error>> {
        transaction.validate()?;
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
    fn validate_stake(&self, stake: &Stake, balance: u128, balance_staked: u128, timestamp: u32) -> Result<(), Box<dyn Error>> {
        stake.validate()?;
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
