use crate::{
    address,
    block::Block,
    constants::{
        BLOCK_STAKES_LIMIT,
        BLOCK_TIME_MAX,
        BLOCK_TIME_MIN,
        BLOCK_TRANSACTIONS_LIMIT,
        MAX_STAKE, // BLOCKS_BEFORE_UNSTAKE
        MIN_STAKE,
        PENDING_STAKES_LIMIT,
        PENDING_TRANSACTIONS_LIMIT,
    },
    db,
    stake::Stake,
    transaction::Transaction,
    types, util,
};
use colored::*;
use log::info;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::{
    collections::{HashMap, VecDeque},
    error::Error,
};
#[derive(Debug)]
pub struct Blockchain {
    latest_block: Block,
    hashes: types::Hashes,
    stakers: types::Stakers,
    pending_transactions: Vec<Transaction>,
    pending_stakes: Vec<Stake>,
    pending_blocks: Vec<Block>,
    sum_stakes_now: types::Amount,
    sum_stakes_all_time: types::Amount,
    balance: types::Balance,
    balance_staked: types::Balance,
}
impl Blockchain {
    pub fn new() -> Blockchain {
        Blockchain {
            latest_block: Block::new([0; 32]),
            hashes: vec![],
            stakers: VecDeque::new(),
            pending_transactions: vec![],
            pending_stakes: vec![],
            pending_blocks: vec![],
            sum_stakes_now: 0,
            sum_stakes_all_time: 0,
            balance: HashMap::new(),
            balance_staked: HashMap::new(),
        }
    }
    pub fn forge_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        keypair: &types::Keypair,
    ) -> Result<Block, Box<dyn Error>> {
        let mut block;
        if let Some(hash) = self.hashes.last() {
            block = Block::new(*hash);
        } else {
            block = Block::new([0; 32]);
        }
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
        block.sign(keypair);
        info!(
            "{}: {} @ {}",
            "Forged".cyan(),
            (self.hashes.len() + 1).to_string().yellow(),
            hex::encode(block.hash()).green()
        );
        self.try_add_block(db, block.clone())?;
        Ok(block)
    }
    pub fn try_add_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        block: Block,
    ) -> Result<(), Box<dyn Error>> {
        if self
            .pending_blocks
            .iter()
            .any(|b| b.signature == block.signature)
        {
            return Err("block already pending".into());
        }
        if !block.is_valid() {
            return Err("block not valid".into());
        }
        if block.previous_hash == [0; 32] {
            println!("previous block was genesis")
        } else if self.hashes.contains(&block.previous_hash) {
            if Block::get(db, &block.hash()).is_ok() {
                return Err("block already in db".into());
            }
        } else {
            return Err("block does not extend chain".into());
        }
        if !self.stakers.is_empty() {
            let previous_block = Block::get(db, &block.previous_hash)?;
            if block.timestamp < previous_block.timestamp + BLOCK_TIME_MIN as types::Timestamp {
                return Err("block created too early".into());
            }
            if block.timestamp > previous_block.timestamp + BLOCK_TIME_MAX as types::Timestamp {
                return Err("block created too late".into());
            }
        }
        // TRANSACTIONS TRANSACTIONS TRANSACTIONS TRANSACTIONS TRANSACTIONS TRANSACTIONS
        for transaction in block.transactions.iter() {
            self.validate_transaction(db, transaction, block.timestamp)?;
        }
        let public_key_inputs = block
            .transactions
            .iter()
            .map(|t| t.public_key_input)
            .collect::<Vec<types::PublicKeyBytes>>();
        if (1..public_key_inputs.len())
            .any(|i| public_key_inputs[i..].contains(&public_key_inputs[i - 1]))
        {
            return Err("block includes multiple transactions from same input".into());
        }
        // STAKES STAKES STAKES STAKES STAKES STAKES STAKES STAKES STAKES STAKES STAKES
        if self.stakers.is_empty() || block.previous_hash == [0; 32] {
            self.validate_mint_stake(&block.stakes, block.timestamp)?;
        } else {
            for stake in block.stakes.iter() {
                self.validate_stake(db, stake, block.timestamp)?;
            }
        }
        let public_keys = block
            .stakes
            .iter()
            .map(|s| s.public_key)
            .collect::<Vec<types::PublicKeyBytes>>();
        if (1..public_keys.len()).any(|i| public_keys[i..].contains(&public_keys[i - 1])) {
            return Err("block includes multiple stakes from same public_key".into());
        }
        // BLOCKS BLOCKS BLOCKS BLOCKS BLOCKS BLOCKS BLOCKS BLOCKS BLOCKS BLOCKS BLOCKS
        if let Some(index) = self
            .pending_blocks
            .iter()
            .position(|b| b.public_key == block.public_key)
        {
            if block.timestamp <= self.pending_blocks[index].timestamp {
                return Err("block is not new enough to replace previous pending block".into());
            }
            self.pending_blocks.remove(index);
        }
        self.pending_blocks.push(block);
        Ok(())
    }
    fn validate_transaction(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        transaction: &Transaction,
        timestamp: types::Timestamp,
    ) -> Result<(), Box<dyn Error>> {
        if !transaction.is_valid() {
            return Err("transaction not valid".into());
        }
        if Transaction::get(db, &transaction.hash()).is_ok() {
            return Err("transaction already in chain".into());
        }
        let balance = self.get_balance(&transaction.public_key_input);
        if transaction.amount + transaction.fee > balance {
            return Err("transaction too expensive".into());
        }
        if transaction.timestamp < timestamp {
            return Err("transaction too old".into());
        }
        Ok(())
    }
    // now only supports 1 transaction per block
    pub fn try_add_transaction(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        transaction: Transaction,
    ) -> Result<(), Box<dyn Error>> {
        if self
            .pending_transactions
            .iter()
            .any(|x| x.signature == transaction.signature)
        {
            return Err("transaction already pending".into());
        }
        self.validate_transaction(db, &transaction, self.latest_block.timestamp)?;
        if let Some(index) = self
            .pending_transactions
            .iter()
            .position(|s| s.public_key_input == transaction.public_key_input)
        {
            if transaction.fee <= self.pending_transactions[index].fee {
                return Err(
                    "transaction fee too low to replace previous pending transaction".into(),
                );
            }
            self.pending_transactions.remove(index);
        }
        self.pending_transactions.push(transaction);
        self.sort_pending_transactions();
        self.limit_pending_transactions();
        Ok(())
    }
    fn validate_mint_stake(
        &self,
        stakes: &Vec<Stake>,
        timestamp: types::Timestamp,
    ) -> Result<(), Box<dyn Error>> {
        if stakes.len() != 1 {
            return Err("only allowed to mint 1 stake".into());
        }
        let stake = stakes.get(0).unwrap();
        if !stake.is_valid() {
            return Err("mint stake not valid".into());
        }
        if stake.timestamp < timestamp {
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
        Ok(())
    }
    fn validate_stake(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        stake: &Stake,
        timestamp: types::Timestamp,
    ) -> Result<(), Box<dyn Error>> {
        if !stake.is_valid() {
            return Err("stake not valid".into());
        }
        if Stake::get(db, &stake.hash()).is_ok() {
            return Err("stake already in chain".into());
        }
        let balance = self.get_balance(&stake.public_key);
        let balance_staked = self.get_balance_staked(&stake.public_key);
        if stake.deposit {
            if stake.amount + stake.fee > balance {
                return Err("stake deposit too expensive".into());
            }
            if stake.amount + balance_staked > MAX_STAKE {
                return Err("stake deposit exceeds MAX_STAKE".into());
            }
        } else {
            if stake.fee > balance {
                return Err("stake withdraw insufficient funds".into());
            }
            if stake.amount > balance_staked {
                return Err("stake withdraw too expensive".into());
            }
        }
        if stake.timestamp < timestamp {
            return Err("stake too old".into());
        }
        Ok(())
    }
    // now only supports 1 stake per block
    pub fn try_add_stake(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        stake: Stake,
    ) -> Result<(), Box<dyn Error>> {
        if self
            .pending_stakes
            .iter()
            .any(|x| x.signature == stake.signature)
        {
            return Err("stake already pending".into());
        }
        self.validate_stake(db, &stake, self.latest_block.timestamp)?;
        if let Some(index) = self
            .pending_stakes
            .iter()
            .position(|s| s.public_key == stake.public_key)
        {
            if stake.fee <= self.pending_stakes[index].fee {
                return Err("stake fee too low to replace previous pending stake".into());
            }
            self.pending_stakes.remove(index);
        }
        self.pending_stakes.push(stake);
        self.sort_pending_stakes();
        self.limit_pending_stakes();
        Ok(())
    }
    pub fn set_mint_stake(&mut self, stake: Stake) {
        self.pending_stakes = vec![stake];
    }
    pub fn height(&self, hash: types::Hash) -> Option<types::Height> {
        self.hashes.iter().position(|&x| x == hash)
    }
    fn set_sum_stakes(&mut self) {
        let mut sum = 0;
        for staker in self.stakers.iter() {
            sum += self.get_balance_staked(&staker.0);
        }
        self.sum_stakes_now = sum;
        self.sum_stakes_all_time += sum;
    }
    fn hashes(
        db: &DBWithThreadMode<SingleThreaded>,
        previous_hash: types::Hash,
    ) -> Result<Vec<types::Hash>, Box<dyn Error>> {
        let mut hashes = vec![];
        let mut previous_hash = previous_hash;
        let mut closure = || -> Result<Option<()>, Box<dyn Error>> {
            match Block::get(db, &previous_hash) {
                Ok(block) => {
                    let hash = block.hash();
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
    pub fn latest_block(
        db: &DBWithThreadMode<SingleThreaded>,
    ) -> Result<Option<Block>, Box<dyn Error>> {
        if let Some(hash) = db.get(db::key(&db::Key::LatestBlockHash))? {
            Ok(Some(Block::get(db, &hash)?))
        } else {
            Ok(None)
        }
    }
    fn put_latest_block_hash(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: types::Hash,
    ) -> Result<(), Box<dyn Error>> {
        db.put(db::key(&db::Key::LatestBlockHash), hash)?;
        Ok(())
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
    fn next_block(&mut self) -> Result<Block, Box<dyn Error>> {
        if self.pending_blocks.is_empty()
            && !self.stakers.is_empty()
            && util::timestamp() > self.latest_block.timestamp + BLOCK_TIME_MAX as types::Timestamp
        {
            self.penalty();
            return Err("validator did not show up 1".into());
        }
        if self.pending_blocks.is_empty() {
            return Err("no pending blocks".into());
        }
        let block;
        if self.stakers.is_empty() {
            block = self.pending_blocks.remove(0);
            self.reward_cold_start(&block);
        } else if let Some(index) = self
            .pending_blocks
            .iter()
            .position(|x| x.public_key == self.stakers[0].0)
        {
            block = self.pending_blocks.remove(index)
        } else {
            self.penalty();
            return Err("validator did not show up 2".into());
        }
        Ok(block)
    }
    pub fn accept_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        forger: bool,
    ) -> Result<(), Box<dyn Error>> {
        let block = self.next_block()?;
        block.put(db)?;
        let hash = block.hash();
        self.hashes.push(hash);
        Blockchain::put_latest_block_hash(db, hash)?;
        self.set_balances(&block);
        self.reward(&block);
        if self.stakers.len() > 1 {
            self.stakers.rotate_left(1);
        }
        for stake in block.stakes.iter() {
            let balance_staked = self.get_balance_staked(&stake.public_key);
            if stake.deposit {
                if balance_staked >= MIN_STAKE
                    && !self.stakers.iter().any(|&e| e.0 == stake.public_key)
                {
                    self.stakers
                        .push_back((stake.public_key, self.hashes.len()));
                }
            } else if balance_staked < MIN_STAKE {
                self.balance_staked.remove(&stake.public_key); // burn low "staked balance" to make sure "staked balance" never exceeds MAX_STAKE after being minted
                                                               // example: A "staked balance" of 0.1 turns into 100.1 after a minted stake.
                log::warn!(
                    "{}: {}",
                    "Burned low balance".red(),
                    address::encode(&stake.public_key)
                );
                let index = self
                    .stakers
                    .iter()
                    .position(|s| s.0 == stake.public_key)
                    .unwrap();
                self.stakers.remove(index).unwrap();
            }
        }
        self.set_sum_stakes();
        self.latest_block = block;
        self.pending_blocks.clear();
        if !forger {
            info!(
                "{}: {} {}",
                "Accepted".green(),
                self.hashes.len().to_string().yellow(),
                hex::encode(hash)
            );
        }
        Ok(())
    }
    fn penalty(&mut self) {
        let public_key = self.stakers[0].0;
        self.balance_staked.remove(&public_key);
        self.stakers.remove(0).unwrap();
        log::warn!("{}: {}", "Burned".red(), address::encode(&public_key));
    }
    fn reward(&mut self, block: &Block) {
        let balance_staked = self.get_balance_staked(&block.public_key);
        let mut balance = self.get_balance(&block.public_key);
        balance += block.reward(balance_staked);
        self.set_balance(block.public_key, balance);
    }
    fn reward_cold_start(&mut self, block: &Block) {
        let mut balance = self.get_balance(&block.public_key);
        balance += MIN_STAKE;
        self.set_balance(block.public_key, balance);
        log::warn!(
            "{}: {} @ {}",
            "Minted".cyan(),
            MIN_STAKE.to_string().yellow(),
            address::encode(&block.public_key).green()
        );
    }
    pub fn reload(&mut self, db: &DBWithThreadMode<SingleThreaded>) {
        self.latest_block = Block::new([0; 32]);
        self.stakers.clear();
        self.hashes.clear();
        self.balance.clear();
        self.balance_staked.clear();
        if let Some(block) = Blockchain::latest_block(db).unwrap() {
            self.latest_block = block;
        }
        let hashes = Blockchain::hashes(db, self.latest_block.hash()).unwrap();
        let timestamp = 0;
        for (index, hash) in hashes.iter().enumerate() {
            let block = Block::get(db, hash).unwrap();
            let mut balance = self.get_balance(&block.public_key);
            let balance_staked = self.get_balance_staked(&block.public_key);
            balance += block.reward(balance_staked);
            // if self.stakers.is_empty() {
            // self.reward_cold_start(&block);
            // }
            if block.timestamp > timestamp + BLOCK_TIME_MAX as types::Timestamp {
                balance += MIN_STAKE;
            }
            self.set_balance(block.public_key, balance);
            self.set_balances(&block);
            self.set_stakers(index, &block);
        }
        self.hashes = hashes;
    }
    pub fn get_balance(&self, public_key: &types::PublicKeyBytes) -> types::Amount {
        match self.balance.get(public_key) {
            Some(b) => *b,
            None => 0,
        }
    }
    pub fn get_balance_staked(&self, public_key: &types::PublicKeyBytes) -> types::Amount {
        match self.balance_staked.get(public_key) {
            Some(b) => *b,
            None => 0,
        }
    }
    pub fn get_latest_block(&self) -> &Block {
        &self.latest_block
    }
    pub fn get_hashes(&self) -> &types::Hashes {
        &self.hashes
    }
    pub fn get_stakers(&self) -> &types::Stakers {
        &self.stakers
    }
    pub fn get_pending_transactions(&self) -> &Vec<Transaction> {
        &self.pending_transactions
    }
    pub fn get_pending_stakes(&self) -> &Vec<Stake> {
        &self.pending_stakes
    }
    pub fn get_pending_blocks(&self) -> &Vec<Block> {
        &self.pending_blocks
    }
    pub fn get_sum_stakes_now(&self) -> &types::Amount {
        &self.sum_stakes_now
    }
    pub fn get_sum_stakes_all_time(&self) -> &types::Amount {
        &self.sum_stakes_all_time
    }
    fn set_balance(&mut self, public_key: types::PublicKeyBytes, balance: types::Amount) {
        self.balance.insert(public_key, balance);
    }
    fn set_balance_staked(
        &mut self,
        public_key: types::PublicKeyBytes,
        balance_staked: types::Amount,
    ) {
        self.balance_staked.insert(public_key, balance_staked);
    }
    fn set_balances(&mut self, block: &Block) {
        for transaction in block.transactions.iter() {
            let mut balance_input = self.get_balance(&transaction.public_key_input);
            let mut balance_output = self.get_balance(&transaction.public_key_output);
            balance_input -= transaction.amount + transaction.fee;
            balance_output += transaction.amount;
            self.set_balance(transaction.public_key_input, balance_input);
            self.set_balance(transaction.public_key_output, balance_output);
        }
        for stake in block.stakes.iter() {
            let mut balance = self.get_balance(&stake.public_key);
            let mut balance_staked = self.get_balance_staked(&stake.public_key);
            if stake.deposit {
                balance -= stake.amount + stake.fee;
                balance_staked += stake.amount;
            } else {
                balance += stake.amount - stake.fee;
                balance_staked -= stake.amount;
            }
            self.set_balance(stake.public_key, balance);
            self.set_balance_staked(stake.public_key, balance_staked);
        }
    }
    fn set_stakers(&mut self, index: usize, block: &Block) {
        for stake in block.stakes.iter() {
            if !self.stakers.iter().any(|&e| e.0 == stake.public_key) {
                self.stakers.push_back((stake.public_key, index));
            }
        }
    }
}
