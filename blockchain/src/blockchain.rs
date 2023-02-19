use crate::state::Dynamic;
use crate::states::States;
use crate::sync::Sync;
use colored::*;
use log::debug;
use log::info;
use log::warn;
use pea_block::BlockA;
use pea_block::BlockB;
use pea_core::*;
use pea_db as db;
use pea_key::Key;
use pea_stake::StakeA;
use pea_stake::StakeB;
use pea_transaction::TransactionA;
use pea_transaction::TransactionB;
use pea_tree::Tree;
use pea_util::EMPTY_BLOCK_SIZE;
use pea_util::STAKE_SIZE;
use pea_util::TRANSACTION_SIZE;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;
#[derive(Debug)]
pub struct Blockchain {
    pub tree: Tree,
    pub states: States,
    pending_transactions: Vec<TransactionA>,
    pending_stakes: Vec<StakeA>,
    pub sync: Sync,
    pub offline: HashMap<AddressBytes, Hash>,
}
impl Blockchain {
    pub fn new() -> Self {
        Self {
            tree: Tree::default(),
            states: States::default(),
            pending_transactions: vec![],
            pending_stakes: vec![],
            sync: Sync::default(),
            offline: HashMap::new(),
        }
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, trust_fork_after_blocks: usize) {
        let start = Instant::now();
        db::tree::reload(&mut self.tree, db);
        info!("Loaded tree in {}", format!("{:?}", start.elapsed()).yellow());
        let start = Instant::now();
        let (hashes_trusted, hashes_dynamic) = self.tree.hashes(trust_fork_after_blocks);
        self.states.trusted.load(db, &hashes_trusted);
        self.states.dynamic = Dynamic::from(db, &hashes_dynamic, &self.states.trusted);
        info!("Loaded states in {}", format!("{:?}", start.elapsed()).yellow());
    }
    pub fn last_seen(&self) -> String {
        if self.states.dynamic.latest_block.timestamp == 0 {
            return "never".to_string();
        }
        let timestamp = self.states.dynamic.latest_block.timestamp;
        let diff = pea_util::timestamp().saturating_sub(timestamp);
        let now = "just now";
        let mut string = pea_util::duration_to_string(diff, now);
        if string != now {
            string.push_str(" ago");
        }
        string
    }
    pub fn height(&self) -> usize {
        self.states.trusted.hashes.len() + self.states.dynamic.hashes.len()
    }
    pub fn sync_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, height: usize) -> Option<BlockB> {
        let hashes_trusted = &self.states.trusted.hashes;
        let hashes_dynamic = &self.states.dynamic.hashes;
        if height >= hashes_trusted.len() + hashes_dynamic.len() {
            return None;
        }
        let hash = if height < hashes_trusted.len() {
            hashes_trusted[height]
        } else {
            hashes_dynamic[height - hashes_trusted.len()]
        };
        debug!("{} {} {}", "Sync".cyan(), height.to_string().yellow(), hex::encode(hash));
        db::block::get_b(db, &hash).ok()
    }
    pub fn forge_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, key: &Key, timestamp: u32, trust_fork_after_blocks: usize) -> Option<BlockA> {
        if let Some(staker) = self.states.dynamic.next_staker(timestamp) {
            if staker != key.address_bytes() || timestamp < self.states.dynamic.latest_block.timestamp + BLOCK_TIME {
                return None;
            }
        } else {
            self.pending_stakes = vec![StakeA::sign(true, 0, 0, timestamp, key).unwrap()];
        }
        let mut transactions: Vec<TransactionA> = self
            .pending_transactions
            .iter()
            .filter(|a| !pea_util::ancient(a.timestamp, timestamp))
            .cloned()
            .collect();
        let mut stakes: Vec<StakeA> = self
            .pending_stakes
            .iter()
            .filter(|a| !pea_util::ancient(a.timestamp, timestamp))
            .cloned()
            .collect();
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
            BlockA::sign(main.0, timestamp, transactions, stakes, key, &self.states.dynamic.latest_block.beta)
        } else {
            BlockA::sign([0; 32], timestamp, transactions, stakes, key, &GENESIS_BETA)
        }
        .unwrap();
        self.save_block(db, &block_a, true, trust_fork_after_blocks);
        Some(block_a)
    }
    pub fn append_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        block_b: BlockB,
        timestamp: u32,
        time_delta: u32,
        trust_fork_after_blocks: usize,
    ) -> Result<(), Box<dyn Error>> {
        let block_a = block_b.a()?;
        self.validate_block(db, &block_a, timestamp, time_delta, trust_fork_after_blocks)?;
        self.save_block(db, &block_a, false, trust_fork_after_blocks);
        Ok(())
    }
    pub fn save_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, block_a: &BlockA, forged: bool, trust_fork_after_blocks: usize) {
        db::block::put(block_a, db).unwrap();
        if self.tree.insert(block_a.hash, block_a.previous_hash, block_a.timestamp).unwrap() {
            warn!("{} {}", "Forked".red(), hex::encode(block_a.hash));
        }
        self.tree.sort_branches();
        self.states
            .update(db, &self.tree.hashes_dynamic(trust_fork_after_blocks), trust_fork_after_blocks);
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
    pub fn pending_transactions_push(&mut self, transaction_b: TransactionB, timestamp: u32, time_delta: u32) -> Result<(), Box<dyn Error>> {
        let transaction_a = transaction_b.a(None)?;
        if self.pending_transactions.iter().any(|x| x.hash == transaction_a.hash) {
            return Err("transaction pending".into());
        }
        let balance = self.balance_available(&transaction_a.input_address);
        if transaction_a.amount + transaction_a.fee > balance {
            return Err("transaction too expensive".into());
        }
        Blockchain::validate_transaction(&self.states.dynamic, &transaction_a, timestamp, time_delta)?;
        info!("Transaction {}", hex::encode(transaction_a.hash).green());
        self.pending_transactions.push(transaction_a);
        self.pending_transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        while *TRANSACTION_SIZE * self.pending_transactions.len() > BLOCK_SIZE_LIMIT - *EMPTY_BLOCK_SIZE {
            self.pending_transactions.remove(self.pending_transactions.len() - 1);
        }
        Ok(())
    }
    pub fn pending_stakes_push(&mut self, stake_b: StakeB, timestamp: u32, time_delta: u32) -> Result<(), Box<dyn Error>> {
        let stake_a = stake_b.a(None)?;
        if self.pending_stakes.iter().any(|x| x.hash == stake_a.hash) {
            return Err("stake pending".into());
        }
        if stake_a.deposit {
            let balance = self.balance_available(&stake_a.input_address);
            if stake_a.amount + stake_a.fee > balance {
                return Err("stake deposit too expensive".into());
            }
        } else {
            let staked = self.staked_available(&stake_a.input_address);
            if stake_a.amount + stake_a.fee > staked {
                return Err("stake withdraw too expensive".into());
            }
        }
        Blockchain::validate_stake(&self.states.dynamic, &stake_a, timestamp, time_delta)?;
        info!("Stake {}", hex::encode(stake_a.hash).green());
        self.pending_stakes.push(stake_a);
        self.pending_stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
        while *STAKE_SIZE * self.pending_stakes.len() > BLOCK_SIZE_LIMIT - *EMPTY_BLOCK_SIZE {
            self.pending_stakes.remove(self.pending_stakes.len() - 1);
        }
        Ok(())
    }
    pub fn validate_block(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        block_a: &BlockA,
        timestamp: u32,
        time_delta: u32,
        trust_fork_after_blocks: usize,
    ) -> Result<(), Box<dyn Error>> {
        if self.tree.get(&block_a.hash).is_some() {
            return Err("block hash in tree".into());
        }
        if block_a.timestamp > timestamp + time_delta {
            return Err("block timestamp future".into());
        }
        if block_a.previous_hash != [0; 32] && self.tree.get(&block_a.previous_hash).is_none() {
            return Err("block previous_hash not in tree".into());
        }
        let input_address = block_a.input_address();
        let dynamic = self.states.dynamic_fork(db, &self.tree, trust_fork_after_blocks, &block_a.previous_hash)?;
        if let Some(hash) = self.offline.get(&input_address) {
            if hash == &block_a.previous_hash {
                return Err("block staker banned".into());
            }
        }
        if block_a.timestamp < dynamic.latest_block.timestamp + BLOCK_TIME {
            return Err("block timestamp early".into());
        }
        let previous_beta = Key::vrf_proof_to_hash(&dynamic.latest_block.pi).unwrap_or(GENESIS_BETA);
        Key::vrf_verify(&block_a.input_public_key, &block_a.pi, &previous_beta).ok_or("invalid proof")?;
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
        for stake_a in block_a.stakes.iter() {
            Blockchain::validate_stake(&dynamic, stake_a, timestamp, time_delta)?;
        }
        for transaction_a in block_a.transactions.iter() {
            Blockchain::validate_transaction(&dynamic, transaction_a, timestamp, time_delta)?;
        }
        dynamic.check_overflow(&block_a.transactions, &block_a.stakes)?;
        Ok(())
    }
    fn validate_transaction(dynamic: &Dynamic, transaction_a: &TransactionA, timestamp: u32, time_delta: u32) -> Result<(), Box<dyn Error>> {
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
        if transaction_a.timestamp > timestamp + time_delta {
            return Err("transaction timestamp future".into());
        }
        if pea_util::ancient(transaction_a.timestamp, dynamic.latest_block.timestamp) {
            return Err("transaction timestamp ancient".into());
        }
        for block in dynamic.non_ancient_blocks.iter() {
            if block.transactions.iter().any(|a| a.hash == transaction_a.hash) {
                return Err("transaction in chain".into());
            }
        }
        Ok(())
    }
    fn validate_stake(dynamic: &Dynamic, stake_a: &StakeA, timestamp: u32, time_delta: u32) -> Result<(), Box<dyn Error>> {
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
        if stake_a.timestamp > timestamp + time_delta {
            return Err("stake timestamp future".into());
        }
        if pea_util::ancient(stake_a.timestamp, dynamic.latest_block.timestamp) {
            return Err("stake timestamp ancient".into());
        }
        for block in dynamic.non_ancient_blocks.iter() {
            if block.stakes.iter().any(|a| a.hash == stake_a.hash) {
                return Err("stake in chain".into());
            }
        }
        Ok(())
    }
    fn balance_available(&self, address: &AddressBytes) -> u128 {
        let mut balance = self.states.dynamic.balance(address);
        for transaction_a in self.pending_transactions.iter() {
            if &transaction_a.input_address == address {
                balance -= transaction_a.amount + transaction_a.fee;
            }
        }
        for stake_a in self.pending_stakes.iter() {
            if &stake_a.input_address == address {
                if stake_a.deposit {
                    balance -= stake_a.amount + stake_a.fee;
                } else {
                    balance -= stake_a.fee;
                }
            }
        }
        balance
    }
    fn staked_available(&self, address: &AddressBytes) -> u128 {
        let mut staked = self.states.dynamic.staked(address);
        for stake_a in self.pending_stakes.iter() {
            if &stake_a.input_address == address {
                if !stake_a.deposit {
                    staked -= stake_a.amount;
                }
            }
        }
        staked
    }
    pub fn balance() {
        unimplemented!()
    }
    pub fn staked() {
        unimplemented!()
    }
    pub fn pending_balance() {
        unimplemented!()
    }
    pub fn pending_staked() {
        unimplemented!()
    }
}
