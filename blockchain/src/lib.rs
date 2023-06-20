use chrono::Utc;
use colored::*;
use lazy_static::lazy_static;
use rocksdb::DB;
use serde::Deserialize;
use serde::Serialize;
use tofuri_block::Block;
use tofuri_fork::Manager;
use tofuri_fork::Stable;
use tofuri_fork::Unstable;
use tofuri_fork::BLOCK_TIME;
use tofuri_key::Key;
use tofuri_stake::Stake;
use tofuri_sync::Sync;
use tofuri_transaction::Transaction;
use tofuri_tree::Tree;
use tofuri_tree::GENESIS_BLOCK_PREVIOUS_HASH;
use tracing::info;
use tracing::instrument;
use tracing::warn;
pub const BLOCK_SIZE_LIMIT: usize = 57797;
pub const GENESIS_BLOCK_BETA: [u8; 32] = [0; 32];
lazy_static! {
    pub static ref EMPTY_BLOCK_SIZE: usize = bincode::serialize(&Block::default()).unwrap().len();
    pub static ref TRANSACTION_SIZE: usize =
        bincode::serialize(&Transaction::default()).unwrap().len();
    pub static ref STAKE_SIZE: usize = bincode::serialize(&Stake::default()).unwrap().len();
}
#[derive(Debug)]
pub enum Error {
    DB(tofuri_db::Error),
    Key(tofuri_key::Error),
    Fork(tofuri_fork::Error),
    BlockPending,
    BlockHashInTree,
    BlockPreviousHashNotInTree,
    BlockTimestampFuture,
    BlockTimestamp,
    BlockStakerAddress,
    TransactionPending,
    TransactionTooExpensive,
    TransactionAmountZero,
    TransactionFeeZero,
    TransactionInputOutput,
    TransactionTimestampFuture,
    TransactionTimestamp,
    TransactionInChain,
    StakePending,
    StakeDepositTooExpensive,
    StakeWithdrawFeeTooExpensive,
    StakeWithdrawAmountTooExpensive,
    StakeAmountZero,
    StakeFeeZero,
    StakeTimestampFuture,
    StakeTimestamp,
    StakeInChain,
    HeightByHash,
    HashByHeight,
    SyncBlock,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Blockchain {
    pub tree: Tree,
    pub forks: Manager,
    pub sync: Sync,
    pending_transactions: Vec<Transaction>,
    pending_stakes: Vec<Stake>,
    pending_blocks: Vec<Block>,
}
impl Blockchain {
    #[instrument(skip_all, level = "debug")]
    pub fn load(&mut self, db: &DB, trust_fork_after_blocks: usize) -> Result<(), Error> {
        tofuri_db::tree::reload(&mut self.tree, db).map_err(Error::DB)?;
        let (mut stable_hashes, unstable_hashes) = self
            .tree
            .stable_and_unstable_hashes(trust_fork_after_blocks);
        let height = self.tree.main().map(|x| x.height);
        info!(
            ?height,
            last_seen = self.last_seen(),
            stable_hashes = stable_hashes.len(),
            unstable_hashes = unstable_hashes.len(),
            tree_size = self.tree.size(),
        );
        if let Ok(checkpoint) = tofuri_db::checkpoint::get(db) {
            info!(checkpoint.height);
            self.forks.stable = Stable::from_checkpoint(
                stable_hashes.drain(..checkpoint.height).collect(),
                checkpoint,
            );
        }
        self.forks.stable.load(db, &stable_hashes);
        self.forks.unstable = Unstable::from(db, &unstable_hashes, &self.forks.stable);
        Ok(())
    }
    pub fn last_seen(&self) -> String {
        if self.forks.unstable.latest_block.timestamp == 0 {
            return "never".to_string();
        }
        let timestamp = self.forks.unstable.latest_block.timestamp;
        let diff = (Utc::now().timestamp() as u32).saturating_sub(timestamp);
        let now = "just now";
        let mut string = duration_to_string(diff, now);
        if string != now {
            string.push_str(" ago");
        }
        string
    }
    pub fn height(&self) -> usize {
        self.forks.stable.hashes.len() + self.forks.unstable.hashes.len()
    }
    pub fn height_by_hash(&self, hash: &[u8; 32]) -> Result<usize, Error> {
        if let Some(index) = self.forks.unstable.hashes.iter().position(|a| a == hash) {
            let height = self.forks.stable.hashes.len() + index + 1;
            return Ok(height);
        }
        if let Some(index) = self.forks.stable.hashes.iter().position(|a| a == hash) {
            let height = index + 1;
            return Ok(height);
        }
        Err(Error::HeightByHash)
    }
    pub fn hash_by_height(&self, height: usize) -> Result<[u8; 32], Error> {
        if height > self.height() {
            return Err(Error::HashByHeight);
        }
        let index = height.saturating_sub(1);
        if index < self.forks.stable.hashes.len() {
            let hash = self.forks.stable.hashes[index];
            Ok(hash)
        } else {
            let hash = self.forks.unstable.hashes[index - self.forks.stable.hashes.len()];
            Ok(hash)
        }
    }
    pub fn sync_block(&mut self, db: &DB, index: usize) -> Result<Block, Error> {
        if index >= self.height() {
            return Err(Error::SyncBlock);
        }
        let hash = if index < self.forks.stable.hashes.len() {
            self.forks.stable.hashes[index]
        } else {
            self.forks.unstable.hashes[index - self.forks.stable.hashes.len()]
        };
        tofuri_db::block::get(db, &hash).map_err(Error::DB)
    }
    pub fn forge_block(
        &mut self,
        db: &DB,
        key: &Key,
        timestamp: u32,
        trust_fork_after_blocks: usize,
    ) -> Block {
        let mut transactions: Vec<Transaction> = self
            .pending_transactions
            .iter()
            .filter(|a| a.timestamp <= timestamp && !self.forks.unstable.transaction_in_chain(a))
            .cloned()
            .collect();
        let mut stakes: Vec<Stake> = self
            .pending_stakes
            .iter()
            .filter(|a| a.timestamp <= timestamp && !self.forks.unstable.stake_in_chain(a))
            .cloned()
            .collect();
        transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
        while *EMPTY_BLOCK_SIZE
            + *TRANSACTION_SIZE * transactions.len()
            + *STAKE_SIZE * stakes.len()
            > BLOCK_SIZE_LIMIT
        {
            match (transactions.last(), stakes.last()) {
                (Some(_), None) => {
                    transactions.pop();
                }
                (None, Some(_)) => {
                    stakes.pop();
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
        let res = self.tree.main();
        let res = match res {
            Some(main) => Block::sign(
                main.hash,
                timestamp,
                transactions,
                stakes,
                key,
                &self.forks.unstable.latest_block.beta().unwrap(),
            ),
            None => Block::sign(
                GENESIS_BLOCK_PREVIOUS_HASH,
                timestamp,
                transactions,
                stakes,
                key,
                &GENESIS_BLOCK_BETA,
            ),
        };
        let block_a = res.unwrap();
        self.save_block(db, &block_a, true, trust_fork_after_blocks);
        block_a
    }
    fn save_block(
        &mut self,
        db: &DB,
        block_a: &Block,
        forger: bool,
        trust_fork_after_blocks: usize,
    ) {
        tofuri_db::block::put(block_a, db).unwrap();
        let fork = self
            .tree
            .insert(block_a.hash(), block_a.previous_hash, block_a.timestamp);
        self.tree.sort_branches();
        if let Some(main) = self.tree.main() {
            if block_a.hash() == main.hash && !forger {
                self.sync.new += 1.0;
            }
        }
        self.forks.update(
            db,
            &self.tree.unstable_hashes(trust_fork_after_blocks),
            trust_fork_after_blocks,
        );
        let height = self.height();
        let hash = hex::encode(block_a.hash());
        let transactions = block_a.transactions.len();
        let stakes = block_a.stakes.len();
        let text = if forger {
            "Forged".magenta()
        } else {
            "Accept".green()
        };
        info!(height, fork, hash, transactions, stakes, "{}", text);
    }
    pub fn save_blocks(&mut self, db: &DB, trust_fork_after_blocks: usize) {
        let timestamp = Utc::now().timestamp() as u32;
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
    pub fn pending_transactions_push(
        &mut self,
        transaction: Transaction,
        time_delta: u32,
    ) -> Result<(), Error> {
        if self
            .pending_transactions
            .iter()
            .any(|x| x.hash() == transaction.hash())
        {
            return Err(Error::TransactionPending);
        }
        if transaction.amount + transaction.fee
            > self
                .balance_pending_min(&transaction.input_address().map_err(Error::Key)?)
                .into()
        {
            return Err(Error::TransactionTooExpensive);
        }
        Blockchain::validate_transaction(
            &self.forks.unstable,
            &transaction,
            Utc::now().timestamp() as u32 + time_delta,
        )?;
        let hash = hex::encode(transaction.hash());
        info!(hash, "Transaction");
        self.pending_transactions.push(transaction);
        Ok(())
    }
    pub fn pending_stakes_push(&mut self, stake: Stake, time_delta: u32) -> Result<(), Error> {
        if self.pending_stakes.iter().any(|x| x.hash() == stake.hash()) {
            return Err(Error::StakePending);
        }
        let balance_pending_min =
            self.balance_pending_min(&stake.input_address().map_err(Error::Key)?);
        if stake.deposit {
            if stake.amount + stake.fee > balance_pending_min.into() {
                return Err(Error::StakeDepositTooExpensive);
            }
        } else {
            if stake.fee > balance_pending_min.into() {
                return Err(Error::StakeWithdrawFeeTooExpensive);
            }
            if stake.amount
                > self
                    .staked_pending_min(&stake.input_address().map_err(Error::Key)?)
                    .into()
            {
                return Err(Error::StakeWithdrawAmountTooExpensive);
            }
        }
        Blockchain::validate_stake(
            &self.forks.unstable,
            &stake,
            Utc::now().timestamp() as u32 + time_delta,
        )?;
        let hash = hex::encode(stake.hash());
        info!(hash, "Stake");
        self.pending_stakes.push(stake);
        Ok(())
    }
    pub fn pending_blocks_push(
        &mut self,
        db: &DB,
        block_a: Block,
        time_delta: u32,
        trust_fork_after_blocks: usize,
    ) -> Result<(), Error> {
        if self
            .pending_blocks
            .iter()
            .any(|a| a.hash() == block_a.hash())
        {
            return Err(Error::BlockPending);
        }
        self.validate_block(
            db,
            &block_a,
            Utc::now().timestamp() as u32 + time_delta,
            trust_fork_after_blocks,
        )?;
        self.pending_blocks.push(block_a);
        Ok(())
    }
    pub fn pending_retain(&mut self, timestamp: u32) {
        self.pending_transactions
            .retain(|a| !tofuri_fork::elapsed(a.timestamp, timestamp));
        self.pending_stakes
            .retain(|a| !tofuri_fork::elapsed(a.timestamp, timestamp));
    }
    fn validate_transaction(
        unstable: &Unstable,
        transaction: &Transaction,
        timestamp: u32,
    ) -> Result<(), Error> {
        if transaction.amount == 0.into() {
            return Err(Error::TransactionAmountZero);
        }
        if transaction.fee == 0.into() {
            return Err(Error::TransactionFeeZero);
        }
        if transaction.input_address().map_err(Error::Key)? == transaction.output_address {
            return Err(Error::TransactionInputOutput);
        }
        if transaction.timestamp > timestamp {
            return Err(Error::TransactionTimestampFuture);
        }
        if tofuri_fork::elapsed(transaction.timestamp, unstable.latest_block.timestamp) {
            return Err(Error::TransactionTimestamp);
        }
        if unstable.transaction_in_chain(transaction) {
            return Err(Error::TransactionInChain);
        }
        Ok(())
    }
    fn validate_stake(unstable: &Unstable, stake: &Stake, timestamp: u32) -> Result<(), Error> {
        if stake.amount == 0.into() {
            return Err(Error::StakeAmountZero);
        }
        if stake.fee == 0.into() {
            return Err(Error::StakeFeeZero);
        }
        if stake.timestamp > timestamp {
            return Err(Error::StakeTimestampFuture);
        }
        if tofuri_fork::elapsed(stake.timestamp, unstable.latest_block.timestamp) {
            return Err(Error::StakeTimestamp);
        }
        if unstable.stake_in_chain(stake) {
            return Err(Error::StakeInChain);
        }
        Ok(())
    }
    pub fn validate_block(
        &self,
        db: &DB,
        block_a: &Block,
        timestamp: u32,
        trust_fork_after_blocks: usize,
    ) -> Result<(), Error> {
        if self.tree.get(&block_a.hash()).is_some() {
            return Err(Error::BlockHashInTree);
        }
        if block_a.previous_hash != GENESIS_BLOCK_PREVIOUS_HASH
            && self.tree.get(&block_a.previous_hash).is_none()
        {
            return Err(Error::BlockPreviousHashNotInTree);
        }
        if block_a.timestamp > timestamp {
            return Err(Error::BlockTimestampFuture);
        }
        let input_address = block_a.input_address().map_err(Error::Key)?;
        let unstable = self
            .forks
            .unstable(
                db,
                &self.tree,
                trust_fork_after_blocks,
                &block_a.previous_hash,
            )
            .map_err(Error::Fork)?;
        if !validate_block_timestamp(block_a.timestamp, unstable.latest_block.timestamp) {
            return Err(Error::BlockTimestamp);
        }
        Key::vrf_verify(
            &block_a.input_public_key().map_err(Error::Key)?,
            &block_a.pi,
            &unstable.latest_block.beta().map_err(Error::Key)?,
        )
        .map_err(Error::Key)?;
        if let Some(staker) = unstable.next_staker(block_a.timestamp) {
            if staker != input_address {
                return Err(Error::BlockStakerAddress);
            }
        }
        for stake in block_a.stakes.iter() {
            Blockchain::validate_stake(&unstable, stake, block_a.timestamp)?;
        }
        for transaction in block_a.transactions.iter() {
            Blockchain::validate_transaction(&unstable, transaction, block_a.timestamp)?;
        }
        unstable
            .check_overflow(&block_a.transactions, &block_a.stakes)
            .map_err(Error::Fork)?;
        Ok(())
    }
    pub fn balance(&self, address: &[u8; 20]) -> u128 {
        self.forks.unstable.balance(address)
    }
    pub fn balance_pending_min(&self, address: &[u8; 20]) -> u128 {
        let mut balance = self.balance(address);
        for transaction in self.pending_transactions.iter() {
            if &transaction.input_address().unwrap() == address {
                balance -= transaction.amount + transaction.fee;
            }
        }
        for stake in self.pending_stakes.iter() {
            if &stake.input_address().unwrap() == address {
                if stake.deposit {
                    balance -= stake.amount;
                    balance -= stake.fee;
                } else {
                    balance -= stake.fee;
                }
            }
        }
        balance
    }
    pub fn balance_pending_max(&self, address: &[u8; 20]) -> u128 {
        let mut balance = self.balance(address);
        for transaction in self.pending_transactions.iter() {
            if &transaction.output_address == address {
                balance += transaction.amount;
            }
        }
        for stake in self.pending_stakes.iter() {
            if &stake.input_address().unwrap() == address && !stake.deposit {
                balance += stake.amount;
                balance -= stake.fee;
            }
        }
        balance
    }
    pub fn staked(&self, address: &[u8; 20]) -> u128 {
        self.forks.unstable.staked(address)
    }
    pub fn staked_pending_min(&self, address: &[u8; 20]) -> u128 {
        let mut staked = self.staked(address);
        for stake in self.pending_stakes.iter() {
            if &stake.input_address().unwrap() == address && !stake.deposit {
                staked -= stake.amount;
            }
        }
        staked
    }
    pub fn staked_pending_max(&self, address: &[u8; 20]) -> u128 {
        let mut staked = self.staked(address);
        for stake in self.pending_stakes.iter() {
            if &stake.input_address().unwrap() == address && stake.deposit {
                staked += stake.amount;
            }
        }
        staked
    }
}
pub fn validate_block_timestamp(timestamp: u32, previous_timestamp: u32) -> bool {
    !(timestamp.saturating_sub(previous_timestamp) == 0 || timestamp % BLOCK_TIME != 0)
}
pub fn duration_to_string(seconds: u32, now: &str) -> String {
    if seconds == 0 {
        return now.to_string();
    }
    let mut string = "".to_string();
    let mut i = 0;
    for (str, num) in [
        ("week", seconds / 604800),
        ("day", seconds / 86400 % 7),
        ("hour", seconds / 3600 % 24),
        ("minute", seconds / 60 % 60),
        ("second", seconds % 60),
    ] {
        if num == 0 {
            continue;
        }
        if i == 1 {
            string.push_str(" and ");
        }
        string.push_str(&format!(
            "{} {}{}",
            num,
            str,
            if num == 1 { "" } else { "s" }
        ));
        if i == 1 {
            break;
        }
        i += 1;
    }
    string
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_block_size_limit() {
        assert_eq!(
            BLOCK_SIZE_LIMIT,
            *EMPTY_BLOCK_SIZE + *TRANSACTION_SIZE * 600
        );
    }
}
