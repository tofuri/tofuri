use colored::*;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tofuri_block::BlockA;
use tofuri_block::BlockB;
use tofuri_core::*;
use tofuri_fork::Manager;
use tofuri_fork::Stable;
use tofuri_fork::Unstable;
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
use tracing::instrument;
use tracing::warn;
#[derive(Debug)]
pub enum Error {
    Block(tofuri_block::Error),
    Transaction(tofuri_transaction::Error),
    Stake(tofuri_stake::Error),
    DBTree(tofuri_db::tree::Error),
    DBBlock(tofuri_db::block::Error),
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
    TransactionAmountFloor,
    TransactionFeeFloor,
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
    StakeAmountFloor,
    StakeFeeFloor,
    StakeTimestampFuture,
    StakeTimestamp,
    StakeInChain,
    HeightByHash,
    HashByHeight,
    SyncBlock,
}
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
    #[instrument(skip_all, level = "debug")]
    pub fn load(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        trust_fork_after_blocks: usize,
    ) -> Result<(), Error> {
        tofuri_db::tree::reload(&mut self.tree, db).map_err(Error::DBTree)?;
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
        let diff = tofuri_util::timestamp().saturating_sub(timestamp);
        let now = "just now";
        let mut string = tofuri_util::duration_to_string(diff, now);
        if string != now {
            string.push_str(" ago");
        }
        string
    }
    pub fn height(&self) -> usize {
        self.forks.stable.hashes.len() + self.forks.unstable.hashes.len()
    }
    pub fn height_by_hash(&self, hash: &Hash) -> Result<usize, Error> {
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
    pub fn hash_by_height(&self, height: usize) -> Result<Hash, Error> {
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
    pub fn sync_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        index: usize,
    ) -> Result<BlockB, Error> {
        if index >= self.height() {
            return Err(Error::SyncBlock);
        }
        let hash = if index < self.forks.stable.hashes.len() {
            self.forks.stable.hashes[index]
        } else {
            self.forks.unstable.hashes[index - self.forks.stable.hashes.len()]
        };
        tofuri_db::block::get_b(db, &hash).map_err(Error::DBBlock)
    }
    pub fn forge_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        key: &Key,
        timestamp: u32,
        trust_fork_after_blocks: usize,
    ) -> BlockA {
        let mut transactions: Vec<TransactionA> = self
            .pending_transactions
            .iter()
            .filter(|a| a.timestamp <= timestamp && !self.forks.unstable.transaction_in_chain(a))
            .cloned()
            .collect();
        let mut stakes: Vec<StakeA> = self
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
            Some(main) => BlockA::sign(
                main.hash,
                timestamp,
                transactions,
                stakes,
                key,
                &self.forks.unstable.latest_block.beta,
            ),
            None => BlockA::sign(
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
        db: &DBWithThreadMode<SingleThreaded>,
        block_a: &BlockA,
        forger: bool,
        trust_fork_after_blocks: usize,
    ) {
        tofuri_db::block::put(block_a, db).unwrap();
        let fork = self
            .tree
            .insert(block_a.hash, block_a.previous_hash, block_a.timestamp);
        self.tree.sort_branches();
        if let Some(main) = self.tree.main() {
            if block_a.hash == main.hash && !forger {
                self.sync.new += 1.0;
            }
        }
        self.forks.update(
            db,
            &self.tree.unstable_hashes(trust_fork_after_blocks),
            trust_fork_after_blocks,
        );
        let height = self.height();
        let hash = hex::encode(block_a.hash);
        let transactions = block_a.transactions.len();
        let stakes = block_a.stakes.len();
        let text = if forger {
            "Forged".magenta()
        } else {
            "Accept".green()
        };
        info!(height, fork, hash, transactions, stakes, "{}", text);
    }
    pub fn save_blocks(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        trust_fork_after_blocks: usize,
    ) {
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
    pub fn pending_transactions_push(
        &mut self,
        transaction_b: TransactionB,
        time_delta: u32,
    ) -> Result<(), Error> {
        let transaction_a = transaction_b.a(None).map_err(Error::Transaction)?;
        if self
            .pending_transactions
            .iter()
            .any(|x| x.hash == transaction_a.hash)
        {
            return Err(Error::TransactionPending);
        }
        if transaction_a.amount + transaction_a.fee
            > self.balance_pending_min(&transaction_a.input_address)
        {
            return Err(Error::TransactionTooExpensive);
        }
        Blockchain::validate_transaction(
            &self.forks.unstable,
            &transaction_a,
            tofuri_util::timestamp() + time_delta,
        )?;
        let hash = hex::encode(transaction_a.hash);
        info!(hash, "Transaction");
        self.pending_transactions.push(transaction_a);
        Ok(())
    }
    pub fn pending_stakes_push(&mut self, stake_b: StakeB, time_delta: u32) -> Result<(), Error> {
        let stake_a = stake_b.a(None).map_err(Error::Stake)?;
        if self.pending_stakes.iter().any(|x| x.hash == stake_a.hash) {
            return Err(Error::StakePending);
        }
        let balance_pending_min = self.balance_pending_min(&stake_a.input_address);
        if stake_a.deposit {
            if stake_a.amount + stake_a.fee > balance_pending_min {
                return Err(Error::StakeDepositTooExpensive);
            }
        } else {
            if stake_a.fee > balance_pending_min {
                return Err(Error::StakeWithdrawFeeTooExpensive);
            }
            if stake_a.amount > self.staked_pending_min(&stake_a.input_address) {
                return Err(Error::StakeWithdrawAmountTooExpensive);
            }
        }
        Blockchain::validate_stake(
            &self.forks.unstable,
            &stake_a,
            tofuri_util::timestamp() + time_delta,
        )?;
        let hash = hex::encode(stake_a.hash);
        info!(hash, "Stake");
        self.pending_stakes.push(stake_a);
        Ok(())
    }
    pub fn pending_blocks_push(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        block_b: BlockB,
        time_delta: u32,
        trust_fork_after_blocks: usize,
    ) -> Result<(), Error> {
        let block_a = block_b.a().map_err(Error::Block)?;
        if self.pending_blocks.iter().any(|a| a.hash == block_a.hash) {
            return Err(Error::BlockPending);
        }
        self.validate_block(
            db,
            &block_a,
            tofuri_util::timestamp() + time_delta,
            trust_fork_after_blocks,
        )?;
        self.pending_blocks.push(block_a);
        Ok(())
    }
    pub fn pending_retain(&mut self, timestamp: u32) {
        self.pending_transactions
            .retain(|a| !tofuri_util::elapsed(a.timestamp, timestamp));
        self.pending_stakes
            .retain(|a| !tofuri_util::elapsed(a.timestamp, timestamp));
    }
    fn validate_transaction(
        unstable: &Unstable,
        transaction_a: &TransactionA,
        timestamp: u32,
    ) -> Result<(), Error> {
        if transaction_a.amount == 0 {
            return Err(Error::TransactionAmountZero);
        }
        if transaction_a.fee == 0 {
            return Err(Error::TransactionFeeZero);
        }
        if transaction_a.amount != tofuri_int::floor(transaction_a.amount) {
            return Err(Error::TransactionAmountFloor);
        }
        if transaction_a.fee != tofuri_int::floor(transaction_a.fee) {
            return Err(Error::TransactionFeeFloor);
        }
        if transaction_a.input_address == transaction_a.output_address {
            return Err(Error::TransactionInputOutput);
        }
        if transaction_a.timestamp > timestamp {
            return Err(Error::TransactionTimestampFuture);
        }
        if tofuri_util::elapsed(transaction_a.timestamp, unstable.latest_block.timestamp) {
            return Err(Error::TransactionTimestamp);
        }
        if unstable.transaction_in_chain(transaction_a) {
            return Err(Error::TransactionInChain);
        }
        Ok(())
    }
    fn validate_stake(unstable: &Unstable, stake_a: &StakeA, timestamp: u32) -> Result<(), Error> {
        if stake_a.amount == 0 {
            return Err(Error::StakeAmountZero);
        }
        if stake_a.fee == 0 {
            return Err(Error::StakeFeeZero);
        }
        if stake_a.amount != tofuri_int::floor(stake_a.amount) {
            return Err(Error::StakeAmountFloor);
        }
        if stake_a.fee != tofuri_int::floor(stake_a.fee) {
            return Err(Error::StakeFeeFloor);
        }
        if stake_a.timestamp > timestamp {
            return Err(Error::StakeTimestampFuture);
        }
        if tofuri_util::elapsed(stake_a.timestamp, unstable.latest_block.timestamp) {
            return Err(Error::StakeTimestamp);
        }
        if unstable.stake_in_chain(stake_a) {
            return Err(Error::StakeInChain);
        }
        Ok(())
    }
    pub fn validate_block(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        block_a: &BlockA,
        timestamp: u32,
        trust_fork_after_blocks: usize,
    ) -> Result<(), Error> {
        if self.tree.get(&block_a.hash).is_some() {
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
        let input_address = block_a.input_address();
        let unstable = self
            .forks
            .unstable(
                db,
                &self.tree,
                trust_fork_after_blocks,
                &block_a.previous_hash,
            )
            .map_err(Error::Fork)?;
        if !tofuri_util::validate_block_timestamp(
            block_a.timestamp,
            unstable.latest_block.timestamp,
        ) {
            return Err(Error::BlockTimestamp);
        }
        Key::vrf_verify(
            &block_a.input_public_key,
            &block_a.pi,
            &unstable.latest_block.beta,
        )
        .map_err(Error::Key)?;
        if let Some(staker) = unstable.next_staker(block_a.timestamp) {
            if staker != input_address {
                return Err(Error::BlockStakerAddress);
            }
        }
        for stake_a in block_a.stakes.iter() {
            Blockchain::validate_stake(&unstable, stake_a, block_a.timestamp)?;
        }
        for transaction_a in block_a.transactions.iter() {
            Blockchain::validate_transaction(&unstable, transaction_a, block_a.timestamp)?;
        }
        unstable
            .check_overflow(&block_a.transactions, &block_a.stakes)
            .map_err(Error::Fork)?;
        Ok(())
    }
    pub fn balance(&self, address: &AddressBytes) -> u128 {
        self.forks.unstable.balance(address)
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
        self.forks.unstable.staked(address)
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
