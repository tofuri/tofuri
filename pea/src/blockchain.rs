use crate::{
    state::Dynamic,
    states::States,
    sync::Sync,
    util::{EMPTY_BLOCK_SIZE, STAKE_SIZE, TRANSACTION_SIZE},
};
use colored::*;
use log::{debug, info, warn};
use pea_block::{BlockA, BlockB};
use pea_core::*;
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
    pub pending_transactions: Vec<TransactionA>,
    pub pending_stakes: Vec<StakeA>,
    pub pending_blocks: Vec<BlockA>,
    pub sync: Sync,
    pub trust_fork_after_blocks: usize,
    pub pending_blocks_limit: usize,
    pub time_delta: u32,
    pub offline: HashMap<AddressBytes, Hash>,
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
    pub fn forge_block(&mut self, timestamp: u32) -> Option<BlockA> {
        if let Some(staker) = self.states.dynamic.next_staker(timestamp) {
            if staker != self.key.address_bytes() || timestamp < self.states.dynamic.latest_block.timestamp + BLOCK_TIME_MIN as u32 {
                return None;
            }
        } else {
            self.pending_stakes = vec![StakeA::sign(true, 0, 0, timestamp, &self.key).unwrap()];
        }
        let mut transactions = self.pending_transactions.clone();
        let mut stakes = self.pending_stakes.clone();
        transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
        while *EMPTY_BLOCK_SIZE + *TRANSACTION_SIZE * transactions.len() + *STAKE_SIZE * stakes.len() > BLOCK_SIZE_LIMIT {
            match (transactions.last(), stakes.last()) {
                (Some(_), None) => {
                    stakes.pop();
                }
                (None, Some(_)) => {
                    transactions.pop();
                }
                (Some(transaction), Some(stake)) => {
                    if transaction.fee < stake.fee {
                        transactions.pop();
                    } else {
                        stakes.pop();
                    }
                }
                _ => unreachable!(),
            }
        }
        let block_a = if let Some(main) = self.tree.main() {
            BlockA::sign(main.0, timestamp, transactions, stakes, &self.key, &self.states.dynamic.latest_block.beta)
        } else {
            BlockA::sign([0; 32], timestamp, transactions, stakes, &self.key, &GENESIS_BETA)
        }
        .unwrap();
        self.accept_block(&block_a, true);
        Some(block_a)
    }
    pub fn accept_block(&mut self, block_a: &BlockA, forged: bool) {
        db::block::put(&block_a, &self.db).unwrap();
        if self.tree.insert(block_a.hash, block_a.previous_hash, block_a.timestamp).unwrap() {
            warn!("{} {}", "Forked".red(), hex::encode(block_a.hash));
        }
        self.tree.sort_branches();
        self.states
            .update(&self.db, &self.tree.hashes_dynamic(self.trust_fork_after_blocks), self.trust_fork_after_blocks);
        let info_0 = if forged { "Forged".magenta() } else { "Accept".green() };
        let info_1 = hex::encode(block_a.hash);
        let info_2 = match block_a.transactions.len() {
            0 => "0".red(),
            x => x.to_string().green(),
        };
        let info_3 = match block_a.stakes.len() {
            0 => "0".red(),
            x => x.to_string().green(),
        };
        if let Some(main) = self.tree.main() {
            if block_a.hash == main.0 {
                self.pending_transactions.clear();
                self.pending_stakes.clear();
                if !forged {
                    self.sync.new += 1.0;
                }
                info!("{} {} {} {} {}", info_0, main.1.to_string().yellow(), info_1, info_2, info_3);
                return;
            }
        }
        info!("{} {} {} {}", info_0, info_1, info_2, info_3);
    }
    pub fn pending_blocks_push(&mut self, block_b: BlockB, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if self.pending_blocks.len() < self.pending_blocks_limit {
            return Err("pending blocks limit reached".into());
        }
        let block_a = block_b.a()?;
        self.validate_block_0(&block_a, timestamp)?;
        self.pending_blocks.push(block_a);
        Ok(())
    }
    pub fn pending_transactions_push(&mut self, transaction_b: TransactionB, timestamp: u32) -> Result<(), Box<dyn Error>> {
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
        while *TRANSACTION_SIZE * self.pending_transactions.len() > BLOCK_SIZE_LIMIT - *EMPTY_BLOCK_SIZE {
            self.pending_transactions.remove(self.pending_transactions.len() - 1);
        }
        Ok(())
    }
    pub fn pending_stakes_push(&mut self, stake_b: StakeB, timestamp: u32) -> Result<(), Box<dyn Error>> {
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
        while *STAKE_SIZE * self.pending_stakes.len() > BLOCK_SIZE_LIMIT - *EMPTY_BLOCK_SIZE {
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
        let input_addresses = block_a.transactions.iter().map(|x| x.input_address).collect::<Vec<AddressBytes>>();
        if (1..input_addresses.len()).any(|i| input_addresses[i..].contains(&input_addresses[i - 1])) {
            return Err("block includes multiple transactions from same input address".into());
        }
        let input_addresses = block_a.stakes.iter().map(|x| x.input_address).collect::<Vec<AddressBytes>>();
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
        if let Some(staker) = dynamic.next_staker(block_a.timestamp) {
            if staker != input_address {
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
            if stake_a.amount != 0 {
                return Err("stake mint amount not zero".into());
            }
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
        if stake_a.amount == 0 {
            return Err("stake amount zero".into());
        }
        if stake_a.fee == 0 {
            return Err("stake fee zero".into());
        }
        if stake_a.amount != pea_int::floor(stake_a.amount) {
            return Err("stake amount floor".into());
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
        if stake_a.deposit {
            if stake_a.amount + stake_a.fee > balance {
                return Err("stake deposit too expensive".into());
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
