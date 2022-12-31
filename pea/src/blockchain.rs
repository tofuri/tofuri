use crate::{state::Dynamic, states::States, sync::Sync};
use colored::*;
use log::{debug, info, warn};
use pea_block::{BlockA, BlockB};
use pea_core::constants::{
    BLOCK_STAKES_LIMIT, BLOCK_TIME_MIN, BLOCK_TRANSACTIONS_LIMIT, GENESIS_BETA, PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT, SYNC_BLOCKS_PER_TICK,
};
use pea_core::{types, util};
use pea_db as db;
use pea_key::Key;
use pea_stake::{StakeA, StakeB};
use pea_transaction::{TransactionA, TransactionB};
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
    pending_transactions: Vec<TransactionA>,
    pending_stakes: Vec<StakeA>,
    pub pending_blocks: Vec<BlockA>,
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
    pub fn sync_block(&mut self) -> BlockB {
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
        let block_b = db::block::get_b(&self.db, &hash).unwrap();
        self.sync.index += 1;
        block_b
    }
    pub fn forge_block(&mut self, timestamp: u32) -> Option<BlockB> {
        if let Some(address) = self.states.dynamic.staker(timestamp) {
            if address != &self.key.address_bytes() || timestamp < self.states.dynamic.latest_block.timestamp + BLOCK_TIME_MIN as u32 {
                return None;
            }
        } else {
            let stake = StakeB::sign(true, 0, timestamp, &self.key).unwrap();
            self.pending_stakes = vec![stake.a(None).unwrap()];
        }
        let mut transactions = vec![];
        let mut stakes = vec![];
        for transaction_a in self.pending_transactions.iter() {
            if transactions.len() < BLOCK_TRANSACTIONS_LIMIT {
                transactions.push(transaction_a.b());
            }
        }
        for stake_a in self.pending_stakes.iter() {
            if stakes.len() < BLOCK_STAKES_LIMIT {
                stakes.push(stake_a.b());
            }
        }
        let block = if let Some(main) = self.tree.main() {
            BlockB::sign(main.0, timestamp, transactions, stakes, &self.key, &self.states.dynamic.latest_block.beta)
        } else {
            BlockB::sign([0; 32], timestamp, transactions, stakes, &self.key, &GENESIS_BETA)
        }
        .unwrap();
        self.accept_block(&block.a(None, None, None, None).unwrap(), true);
        Some(block)
    }
    pub fn accept_block(&mut self, block: &BlockA, forged: bool) {
        db::block::put(&block, &self.db).unwrap();
        if self.tree.insert(block.hash, block.previous_hash, block.timestamp).unwrap() {
            warn!("{} {}", "Forked".red(), hex::encode(block.hash));
        }
        self.tree.sort_branches();
        self.states
            .update(&self.db, &self.tree.hashes_dynamic(self.trust_fork_after_blocks), self.trust_fork_after_blocks);
        let info_0 = if forged { "Forged".magenta() } else { "Accept".green() };
        let info_1 = hex::encode(block.hash);
        if let Some(main) = self.tree.main() {
            if block.hash == main.0 {
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
    pub fn add_block(&mut self, block_b: BlockB, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if self.pending_blocks.len() < self.pending_blocks_limit {
            return Err("pending blocks limit reached".into());
        }
        let block_a = block_b.a(None, None, None, None)?;
        self.validate_block_0(&block_a, timestamp)?;
        self.pending_blocks.push(block_a);
        Ok(())
    }
    pub fn add_transaction(&mut self, transaction_b: TransactionB, timestamp: u32) -> Result<(), Box<dyn Error>> {
        let transaction_a = transaction_b.a(None)?;
        self.validate_transaction(&transaction_a, self.states.dynamic.latest_block.timestamp, timestamp)?;
        if self.pending_transactions.iter().any(|x| x.hash == transaction_a.hash) {
            return Err("transaction pending".into());
        }
        if let Some(index) = self.pending_transactions.iter().position(|x| x.input_address == transaction_a.input_address) {
            if transaction_a.fee <= self.pending_transactions[index].fee {
                return Err("transaction fee too low".into());
            }
            self.pending_transactions.remove(index);
        }
        info!("Transaction {}", hex::encode(&transaction_a.hash).green());
        self.pending_transactions.push(transaction_a);
        self.pending_transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        while self.pending_transactions.len() > PENDING_TRANSACTIONS_LIMIT {
            self.pending_transactions.remove(self.pending_transactions.len() - 1);
        }
        Ok(())
    }
    pub fn add_stake(&mut self, stake_b: StakeB, timestamp: u32) -> Result<(), Box<dyn Error>> {
        let stake_a = stake_b.a(None)?;
        self.validate_stake(&stake_a, self.states.dynamic.latest_block.timestamp, timestamp)?;
        if self.pending_stakes.iter().any(|x| x.hash == stake_a.hash) {
            return Err("stake pending".into());
        }
        if let Some(index) = self.pending_stakes.iter().position(|x| x.input_address == stake_a.input_address) {
            if stake_a.fee <= self.pending_stakes[index].fee {
                return Err("stake fee too low".into());
            }
            self.pending_stakes.remove(index);
        }
        info!("Stake {}", hex::encode(&stake_a.hash).green());
        self.pending_stakes.push(stake_a);
        self.pending_stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
        while self.pending_stakes.len() > PENDING_STAKES_LIMIT {
            self.pending_stakes.remove(self.pending_stakes.len() - 1);
        }
        Ok(())
    }
    pub fn validate_block_0(&self, block_a: &BlockA, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if self.tree.get(&block_a.hash).is_some() {
            return Err("block hash in tree".into());
        }
        if block_a.timestamp > timestamp + self.time_delta {
            return Err("block timestamp future".into());
        }
        if block_a.previous_hash != [0; 32] && self.tree.get(&block_a.previous_hash).is_none() {
            return Err("block previous_hash not in tree".into());
        }
        let dynamic = self.states.dynamic_fork(self, &block_a.previous_hash)?;
        let previous_beta = match Key::vrf_proof_to_hash(&dynamic.latest_block.pi) {
            Some(x) => x,
            None => GENESIS_BETA,
        };
        Key::vrf_verify(&block_a.input_public_key, &block_a.pi, &previous_beta).ok_or("invalid proof")?;
        for stake_a in block_a.stakes.iter() {
            self.validate_stake(stake_a, dynamic.latest_block.timestamp, timestamp)?;
        }
        for transaction_a in block_a.transactions.iter() {
            self.validate_transaction(transaction_a, dynamic.latest_block.timestamp, timestamp)?;
        }
        let input_addresses = block_a.transactions.iter().map(|x| x.input_address).collect::<Vec<types::AddressBytes>>();
        if (1..input_addresses.len()).any(|i| input_addresses[i..].contains(&input_addresses[i - 1])) {
            return Err("block includes multiple transactions from same input address".into());
        }
        let input_addresses = block_a.stakes.iter().map(|x| x.input_address).collect::<Vec<types::AddressBytes>>();
        if (1..input_addresses.len()).any(|i| input_addresses[i..].contains(&input_addresses[i - 1])) {
            return Err("block includes multiple stakes from same input address".into());
        }
        Ok(())
    }
    pub fn validate_block_1(&self, block_a: &BlockA) -> Result<(), Box<dyn Error>> {
        let input_address = block_a.input_address();
        let dynamic = self.states.dynamic_fork(self, &block_a.previous_hash)?;
        if let Some(hash) = self.offline.get(&input_address) {
            if hash == &block_a.previous_hash {
                return Err("block staker banned".into());
            }
        }
        if block_a.timestamp < dynamic.latest_block.timestamp + BLOCK_TIME_MIN as u32 {
            return Err("block timestamp early".into());
        }
        if let Some(a) = dynamic.staker(block_a.timestamp) {
            if a != &input_address {
                return Err("block staker address".into());
            }
        } else {
            if !block_a.transactions.is_empty() {
                return Err("block mint transactions".into());
            }
            if block_a.stakes.len() != 1 {
                return Err("block mint stakes".into());
            }
            let stake_a = block_a.stakes.first().unwrap();
            if stake_a.fee != 0 {
                return Err("stake mint fee not zero".into());
            }
            if !stake_a.deposit {
                return Err("stake mint deposit".into());
            }
            if stake_a.timestamp != block_a.timestamp {
                return Err("stake mint timestamp".into());
            }
            return Ok(());
        }
        Ok(())
    }
    fn validate_transaction(&self, transaction_a: &TransactionA, previous_block_timestamp: u32, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if transaction_a.amount == 0 {
            return Err("transaction amount zero".into());
        }
        if transaction_a.fee == 0 {
            return Err("transaction fee zero".into());
        }
        if transaction_a.amount != pea_int::floor(transaction_a.amount) {
            return Err("transaction amount floor".into());
        }
        if transaction_a.fee != pea_int::floor(transaction_a.fee) {
            return Err("transaction fee floor".into());
        }
        if transaction_a.input_address == transaction_a.output_address {
            return Err("transaction input output".into());
        }
        let balance = self.states.dynamic.balance(&transaction_a.input_address);
        if transaction_a.timestamp > timestamp + self.time_delta {
            return Err("transaction timestamp future".into());
        }
        if transaction_a.timestamp < previous_block_timestamp {
            return Err("transaction timestamp ancient".into());
        }
        if transaction_a.amount + transaction_a.fee > balance {
            return Err("transaction too expensive".into());
        }
        if db::transaction::get_b(&self.db, &transaction_a.hash).is_ok() {
            return Err("transaction in chain".into());
        }
        Ok(())
    }
    fn validate_stake(&self, stake_a: &StakeA, previous_block_timestamp: u32, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if stake_a.fee == 0 {
            return Err("stake fee zero".into());
        }
        if stake_a.fee != pea_int::floor(stake_a.fee) {
            return Err("stake fee floor".into());
        }
        if stake_a.timestamp > timestamp + self.time_delta {
            return Err("stake timestamp future".into());
        }
        if stake_a.timestamp < previous_block_timestamp {
            return Err("stake timestamp ancient".into());
        }
        let balance = self.states.dynamic.balance(&stake_a.input_address);
        let balance_staked = self.states.dynamic.balance_staked(&stake_a.input_address);
        if stake_a.deposit {
            if util::stake_amount(self.states.dynamic.stakers.len()) + stake_a.fee > balance {
                return Err("stake deposit too expensive".into());
            }
            if balance_staked != 0 {
                return Err("already staking".into());
            }
        } else if stake_a.fee > balance {
            return Err("stake withdraw fee too expensive".into());
        }
        if db::stake::get_b(&self.db, &stake_a.hash).is_ok() {
            return Err("stake in chain".into());
        }
        Ok(())
    }
}
