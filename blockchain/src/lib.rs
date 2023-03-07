use colored::*;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use std::error::Error;
use tofuri_block::BlockA;
use tofuri_block::BlockB;
use tofuri_core::*;
use tofuri_fork::ForkA;
use tofuri_fork::Manager;
use tofuri_key::Key;
use tofuri_stake::StakeA;
use tofuri_stake::StakeB;
use tofuri_sync::Sync;
use tofuri_transaction::TransactionA;
use tofuri_transaction::TransactionB;
use tofuri_tree::Tree;
use tofuri_util::EMPTY_BLOCK_SIZE;
use tofuri_util::STAKE_SIZE;
use tofuri_util::TRANSACTION_SIZE;
use tracing::info;
#[derive(Default, Debug, Clone)]
pub struct Blockchain {
    pub tree: Tree,
    pub forks: Manager,
    pub sync: Sync,
    pending_transactions: Vec<TransactionA>,
    pending_stakes: Vec<StakeA>,
    pending_blocks: Vec<BlockA>,
}
impl Blockchain {
    #[tracing::instrument(skip_all, level = "debug")]
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, trust_fork_after_blocks: usize) {
        tofuri_db::tree::reload(&mut self.tree, db);
        let (hashes_trusted, hashes_dynamic) = self.tree.hashes(trust_fork_after_blocks);
        self.forks.b.load(db, &hashes_trusted);
        self.forks.a = ForkA::from(db, &hashes_dynamic, &self.forks.b);
        let height = if let Some(main) = self.tree.main() { main.height } else { 0 };
        let last_seen = self.last_seen();
        info!(height, last_seen);
    }
    pub fn last_seen(&self) -> String {
        if self.forks.a.latest_block.timestamp == 0 {
            return "never".to_string();
        }
        let timestamp = self.forks.a.latest_block.timestamp;
        let diff = tofuri_util::timestamp().saturating_sub(timestamp);
        let now = "just now";
        let mut string = tofuri_util::duration_to_string(diff, now);
        if string != now {
            string.push_str(" ago");
        }
        string
    }
    pub fn height(&self) -> usize {
        self.forks.b.hashes.len() + self.forks.a.hashes.len()
    }
    pub fn height_by_hash(&self, hash: &Hash) -> Option<usize> {
        if let Some(index) = self.forks.a.hashes.iter().position(|a| a == hash) {
            return Some(self.forks.b.hashes.len() + index);
        }
        self.forks.b.hashes.iter().position(|a| a == hash)
    }
    pub fn hash_by_height(&self, height: usize) -> Option<Hash> {
        let len_trusted = self.forks.b.hashes.len();
        let len_dynamic = self.forks.a.hashes.len();
        if height >= len_trusted + len_dynamic {
            return None;
        }
        if height < len_trusted {
            Some(self.forks.b.hashes[height])
        } else {
            Some(self.forks.a.hashes[height - len_trusted])
        }
    }
    pub fn sync_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, height: usize) -> Option<BlockB> {
        let hashes_trusted = &self.forks.b.hashes;
        let hashes_dynamic = &self.forks.a.hashes;
        if height >= hashes_trusted.len() + hashes_dynamic.len() {
            return None;
        }
        let hash = if height < hashes_trusted.len() {
            hashes_trusted[height]
        } else {
            hashes_dynamic[height - hashes_trusted.len()]
        };
        tofuri_db::block::get_b(db, &hash).ok()
    }
    pub fn forge_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, key: &Key, timestamp: u32, trust_fork_after_blocks: usize) -> BlockA {
        if self.forks.a.next_staker(timestamp).is_none() {
            self.pending_stakes = vec![StakeA::sign(true, 0, 0, timestamp, key).unwrap()];
        }
        let mut transactions: Vec<TransactionA> = self
            .pending_transactions
            .iter()
            .filter(|a| a.timestamp <= timestamp && !self.forks.a.transaction_in_chain(a))
            .cloned()
            .collect();
        let mut stakes: Vec<StakeA> = self
            .pending_stakes
            .iter()
            .filter(|a| a.timestamp <= timestamp && !self.forks.a.stake_in_chain(a))
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
            BlockA::sign(main.hash, timestamp, transactions, stakes, key, &self.forks.a.latest_block.beta)
        } else {
            BlockA::sign(GENESIS_BLOCK_PREVIOUS_HASH, timestamp, transactions, stakes, key, &GENESIS_BLOCK_BETA)
        }
        .unwrap();
        self.save_block(db, &block_a, true, trust_fork_after_blocks);
        block_a
    }
    fn save_block(&mut self, db: &DBWithThreadMode<SingleThreaded>, block_a: &BlockA, forger: bool, trust_fork_after_blocks: usize) {
        tofuri_db::block::put(block_a, db).unwrap();
        let fork = self.tree.insert(block_a.hash, block_a.previous_hash, block_a.timestamp).unwrap();
        self.tree.sort_branches();
        if let Some(main) = self.tree.main() {
            if block_a.hash == main.hash && !forger {
                self.sync.new += 1.0;
            }
        }
        self.forks
            .update(db, &self.tree.hashes_dynamic(trust_fork_after_blocks), trust_fork_after_blocks);
        info!(
            height = self.height(),
            fork,
            hash = hex::encode(block_a.hash),
            transactions = block_a.transactions.len(),
            stakes = block_a.stakes.len(),
            "{}",
            if forger { "Forged".magenta() } else { "Accept".green() }
        );
    }
    pub fn save_blocks(&mut self, db: &DBWithThreadMode<SingleThreaded>, trust_fork_after_blocks: usize) {
        let timestamp = tofuri_util::timestamp();
        let mut vec = vec![];
        let mut i = 0;
        while i < self.pending_blocks.len() {
            if self.pending_blocks[i].timestamp <= timestamp {
                vec.push(self.pending_blocks.remove(i));
            } else {
                i += 1;
            }
        }
        for block_a in vec {
            self.save_block(db, &block_a, false, trust_fork_after_blocks);
        }
    }
    pub fn pending_transactions_push(&mut self, transaction_b: TransactionB, time_delta: u32) -> Result<(), Box<dyn Error>> {
        let transaction_a = transaction_b.a(None)?;
        if self.pending_transactions.iter().any(|x| x.hash == transaction_a.hash) {
            return Err("transaction pending".into());
        }
        if transaction_a.amount + transaction_a.fee > self.balance_pending_min(&transaction_a.input_address) {
            return Err("transaction too expensive".into());
        }
        Blockchain::validate_transaction(&self.forks.a, &transaction_a, tofuri_util::timestamp() + time_delta)?;
        info!(hash = hex::encode(transaction_a.hash), "Transaction");
        self.pending_transactions.push(transaction_a);
        Ok(())
    }
    pub fn pending_stakes_push(&mut self, stake_b: StakeB, time_delta: u32) -> Result<(), Box<dyn Error>> {
        let stake_a = stake_b.a(None)?;
        if self.pending_stakes.iter().any(|x| x.hash == stake_a.hash) {
            return Err("stake pending".into());
        }
        let balance_pending_min = self.balance_pending_min(&stake_a.input_address);
        if stake_a.deposit {
            if stake_a.amount + stake_a.fee > balance_pending_min {
                return Err("stake deposit too expensive".into());
            }
        } else {
            if stake_a.fee > balance_pending_min {
                return Err("stake withdraw fee too expensive".into());
            }
            if stake_a.amount > self.staked_pending_min(&stake_a.input_address) {
                return Err("stake withdraw amount too expensive".into());
            }
        }
        Blockchain::validate_stake(&self.forks.a, &stake_a, tofuri_util::timestamp() + time_delta)?;
        info!(hash = hex::encode(stake_a.hash), "Stake");
        self.pending_stakes.push(stake_a);
        Ok(())
    }
    pub fn pending_blocks_push(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        block_b: BlockB,
        time_delta: u32,
        trust_fork_after_blocks: usize,
    ) -> Result<(), Box<dyn Error>> {
        let block_a = block_b.a()?;
        if self.pending_blocks.iter().any(|a| a.hash == block_a.hash) {
            return Err("block pending".into());
        }
        self.validate_block(db, &block_a, tofuri_util::timestamp() + time_delta, trust_fork_after_blocks)?;
        self.pending_blocks.push(block_a);
        Ok(())
    }
    pub fn pending_retain_non_ancient(&mut self, timestamp: u32) {
        self.pending_transactions.retain(|a| !tofuri_util::ancient(a.timestamp, timestamp));
        self.pending_stakes.retain(|a| !tofuri_util::ancient(a.timestamp, timestamp));
    }
    fn validate_transaction(dynamic: &ForkA, transaction_a: &TransactionA, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if transaction_a.amount == 0 {
            return Err("transaction amount zero".into());
        }
        if transaction_a.fee == 0 {
            return Err("transaction fee zero".into());
        }
        if transaction_a.amount != tofuri_int::floor(transaction_a.amount) {
            return Err("transaction amount floor".into());
        }
        if transaction_a.fee != tofuri_int::floor(transaction_a.fee) {
            return Err("transaction fee floor".into());
        }
        if transaction_a.input_address == transaction_a.output_address {
            return Err("transaction input output".into());
        }
        if transaction_a.timestamp > timestamp {
            return Err("transaction timestamp future".into());
        }
        if tofuri_util::ancient(transaction_a.timestamp, dynamic.latest_block.timestamp) {
            return Err("transaction timestamp ancient".into());
        }
        if dynamic.transaction_in_chain(transaction_a) {
            return Err("transaction in chain".into());
        }
        Ok(())
    }
    fn validate_stake(dynamic: &ForkA, stake_a: &StakeA, timestamp: u32) -> Result<(), Box<dyn Error>> {
        if stake_a.amount == 0 {
            return Err("stake amount zero".into());
        }
        if stake_a.fee == 0 {
            return Err("stake fee zero".into());
        }
        if stake_a.amount != tofuri_int::floor(stake_a.amount) {
            return Err("stake amount floor".into());
        }
        if stake_a.fee != tofuri_int::floor(stake_a.fee) {
            return Err("stake fee floor".into());
        }
        if stake_a.timestamp > timestamp {
            return Err("stake timestamp future".into());
        }
        if tofuri_util::ancient(stake_a.timestamp, dynamic.latest_block.timestamp) {
            return Err("stake timestamp ancient".into());
        }
        if dynamic.stake_in_chain(stake_a) {
            return Err("stake in chain".into());
        }
        Ok(())
    }
    pub fn validate_block(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        block_a: &BlockA,
        timestamp: u32,
        trust_fork_after_blocks: usize,
    ) -> Result<(), Box<dyn Error>> {
        if self.tree.get(&block_a.hash).is_some() {
            return Err("block hash in tree".into());
        }
        if block_a.timestamp > timestamp {
            return Err("block timestamp future".into());
        }
        if block_a.previous_hash != GENESIS_BLOCK_PREVIOUS_HASH && self.tree.get(&block_a.previous_hash).is_none() {
            return Err("block previous_hash not in tree".into());
        }
        let input_address = block_a.input_address();
        let dynamic = self.forks.dynamic_fork(db, &self.tree, trust_fork_after_blocks, &block_a.previous_hash)?;
        let previous_beta = Key::vrf_proof_to_hash(&dynamic.latest_block.pi).unwrap_or(GENESIS_BLOCK_BETA);
        Key::vrf_verify(&block_a.input_public_key, &block_a.pi, &previous_beta).ok_or("invalid proof")?;
        if let Some(staker) = dynamic.next_staker(block_a.timestamp) {
            if staker != input_address {
                return Err("block staker address".into());
            }
            if block_a.timestamp != dynamic.latest_block.timestamp + BLOCK_TIME {
                return Err("block timestamp".into());
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
            Blockchain::validate_stake(&dynamic, stake_a, block_a.timestamp)?;
        }
        for transaction_a in block_a.transactions.iter() {
            Blockchain::validate_transaction(&dynamic, transaction_a, block_a.timestamp)?;
        }
        dynamic.check_overflow(&block_a.transactions, &block_a.stakes)?;
        Ok(())
    }
    pub fn balance(&self, address: &AddressBytes) -> u128 {
        self.forks.a.balance(address)
    }
    pub fn balance_pending_min(&self, address: &AddressBytes) -> u128 {
        let mut balance = self.balance(address);
        for transaction_a in self.pending_transactions.iter() {
            if &transaction_a.input_address == address {
                balance -= transaction_a.amount + transaction_a.fee;
            }
        }
        for stake_a in self.pending_stakes.iter() {
            if &stake_a.input_address == address {
                if stake_a.deposit {
                    balance -= stake_a.amount;
                    balance -= stake_a.fee;
                } else {
                    balance -= stake_a.fee;
                }
            }
        }
        balance
    }
    pub fn balance_pending_max(&self, address: &AddressBytes) -> u128 {
        let mut balance = self.balance(address);
        for transaction_a in self.pending_transactions.iter() {
            if &transaction_a.output_address == address {
                balance += transaction_a.amount;
            }
        }
        for stake_a in self.pending_stakes.iter() {
            if &stake_a.input_address == address && !stake_a.deposit {
                balance += stake_a.amount;
                balance -= stake_a.fee;
            }
        }
        balance
    }
    pub fn staked(&self, address: &AddressBytes) -> u128 {
        self.forks.a.staked(address)
    }
    pub fn staked_pending_min(&self, address: &AddressBytes) -> u128 {
        let mut staked = self.staked(address);
        for stake_a in self.pending_stakes.iter() {
            if &stake_a.input_address == address && !stake_a.deposit {
                staked -= stake_a.amount;
            }
        }
        staked
    }
    pub fn staked_pending_max(&self, address: &AddressBytes) -> u128 {
        let mut staked = self.staked(address);
        for stake_a in self.pending_stakes.iter() {
            if &stake_a.input_address == address && stake_a.deposit {
                staked += stake_a.amount;
            }
        }
        staked
    }
}
