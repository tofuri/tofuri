use crate::{
    address,
    block::Block,
    constants::{
        BLOCK_STAKES_LIMIT, BLOCK_TIME_MAX, BLOCK_TRANSACTIONS_LIMIT, MIN_STAKE,
        PENDING_BLOCKS_LIMIT, PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT,
    },
    db,
    stake::Stake,
    transaction::Transaction,
    types, util,
};
use colored::*;
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
    stakers_history: types::StakersHistory,
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
            stakers_history: HashMap::new(),
        }
    }
    fn height(&self, hash: types::Hash) -> Option<types::Height> {
        self.hashes.iter().position(|&x| x == hash)
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
                            "{} {} != {}",
                            "Detected broken chain!".red(),
                            hex::encode(hash),
                            hex::encode(previous_hash)
                        );
                        log::warn!("{}", "Pruning broken chain".yellow());
                        hashes.clear();
                        Blockchain::set_latest_block_hash(db, previous_hash)?;
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
        hashes.reverse();
        Ok(hashes)
    }
    fn penalty(&mut self) {
        let public_key = self.stakers[0].0;
        self.balance_staked.remove(&public_key);
        self.stakers.remove(0).unwrap();
        log::warn!("{} {}", "Burned".red(), address::encode(&public_key));
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
            "{} {} {}",
            "Minted".cyan(),
            MIN_STAKE.to_string().yellow(),
            address::encode(&block.public_key).green()
        );
    }
    pub fn pending_blocks_push(
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
        block.validate(self, db)?;
        self.pending_blocks.push(block);
        self.limit_pending_blocks();
        Ok(())
    }
    pub fn pending_transactions_push(
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
        transaction.validate(self, db, self.latest_block.timestamp)?;
        self.pending_transactions.push(transaction);
        self.limit_pending_transactions();
        Ok(())
    }
    pub fn pending_stakes_push(
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
        stake.validate(self, db, self.latest_block.timestamp)?;
        self.pending_stakes.push(stake);
        self.limit_pending_stakes();
        Ok(())
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
        for transaction in self.pending_transactions.iter() {
            if block.transactions.len() < BLOCK_TRANSACTIONS_LIMIT {
                block.transactions.push(transaction.clone());
            }
        }
        self.sort_pending_stakes();
        for stake in self.pending_stakes.iter() {
            if block.stakes.len() < BLOCK_STAKES_LIMIT {
                block.stakes.push(stake.clone());
            }
        }
        block.sign(keypair);
        let hash = self.append(db, block.clone(), true).unwrap();
        log::info!(
            "{} {} {}",
            "Forged".green(),
            self.get_height().to_string().yellow(),
            hex::encode(hash)
        );
        Ok(block)
    }
    pub fn try_append_loop(&mut self, db: &DBWithThreadMode<SingleThreaded>) {
        for block in self.pending_blocks.clone() {
            self.try_append(db, block);
        }
        if util::timestamp() > self.latest_block.timestamp + BLOCK_TIME_MAX as types::Timestamp {
            self.penalty();
            log::error!("staker didn't show up in time");
        } else {
            log::info!("no pending blocks, waiting...");
        }
    }
    pub fn try_append(&mut self, db: &DBWithThreadMode<SingleThreaded>, block: Block) {
        if let Some(public_key) = self.stakers_history.get(&block.previous_hash) {
            if public_key != &block.public_key {
                println!("block public_key don't match stakers history");
                return;
            }
        } else {
            println!("block didn't have a staker because network was down");
        }
        let hash;
        if block.previous_hash == self.latest_block.hash()
            || self.latest_block.previous_hash == [0; 32]
        {
            hash = self.append(db, block, true).unwrap();
        } else {
            hash = self.append(db, block, false).unwrap();
        }
        log::info!(
            "{} {} {}",
            "Accepted".green(),
            self.get_height().to_string().yellow(),
            hex::encode(hash)
        );
    }
    pub fn append(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        block: Block,
        latest: bool,
    ) -> Result<types::Hash, Box<dyn Error>> {
        block.put(db)?;
        let hash = block.hash();
        if latest {
            Blockchain::set_latest_block_hash(db, hash)?;
            self.hashes.push(hash);
            if self.stakers.is_empty() {
                self.reward_cold_start(&block);
            } else {
                self.reward(&block);
            }
            self.set_balances(&block);
            self.set_stakers(self.get_height(), &block);
            self.set_sum_stakes();
            self.latest_block = block;
            self.pending_blocks.clear();
            self.pending_transactions.clear();
            self.pending_stakes.clear();
        } else {
            if let Some(height) = self.height(block.previous_hash) {
                if height + 1 > self.get_height() {
                    log::warn!("Fork detected! Reloading...");
                    self.reload(db);
                }
            }
        }
        Ok(hash)
    }
    pub fn reload(&mut self, db: &DBWithThreadMode<SingleThreaded>) {
        self.latest_block = Block::new([0; 32]);
        self.stakers.clear();
        self.hashes.clear();
        self.balance.clear();
        self.balance_staked.clear();
        self.stakers_history.clear();
        self.set_latest_block(db).unwrap();
        let hashes = Blockchain::hashes(db, self.latest_block.hash()).unwrap();
        let mut previous_block_timestamp = match hashes.first() {
            Some(hash) => Block::get(db, hash).unwrap().timestamp - 1,
            None => 0,
        };
        for (height, hash) in hashes.iter().enumerate() {
            let block = Block::get(db, hash).unwrap();
            let diff = block.timestamp - previous_block_timestamp - 1;
            for _ in 0..diff / BLOCK_TIME_MAX as u32 {
                if !self.stakers.is_empty() {
                    self.penalty();
                }
            }
            if self.stakers.is_empty() {
                self.reward_cold_start(&block);
            } else {
                self.reward(&block);
            }
            self.set_balances(&block);
            self.set_stakers(height, &block);
            self.set_sum_stakes();
            previous_block_timestamp = block.timestamp;
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
    pub fn get_height(&self) -> types::Height {
        if self.hashes.is_empty() {
            return 0;
        }
        self.hashes.len() - 1
    }
    pub fn get_stakers_history(&self) -> &types::StakersHistory {
        &self.stakers_history
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
    fn set_stakers(&mut self, height: usize, block: &Block) {
        if self.stakers.len() > 1 {
            self.stakers.rotate_left(1);
        }
        for stake in block.stakes.iter() {
            let balance_staked = self.get_balance_staked(&stake.public_key);
            let any = self.stakers.iter().any(|&e| e.0 == stake.public_key);
            if !any && balance_staked >= MIN_STAKE {
                self.stakers.push_back((stake.public_key, height));
            } else if any && balance_staked < MIN_STAKE {
                self.balance_staked.remove(&stake.public_key);
                let index = self
                    .stakers
                    .iter()
                    .position(|staker| staker.0 == stake.public_key)
                    .unwrap();
                self.stakers.remove(index).unwrap();
                log::warn!(
                    "{} {}",
                    "Burned low balance".red(),
                    address::encode(&stake.public_key)
                );
            }
        }
        if let Some(staker) = self.stakers.get(0) {
            self.stakers_history.insert(block.hash(), staker.0);
        }
    }
    pub fn set_cold_start_stake(&mut self, stake: Stake) {
        self.pending_stakes = vec![stake];
    }
    fn set_sum_stakes(&mut self) {
        let mut sum = 0;
        for staker in self.stakers.iter() {
            sum += self.get_balance_staked(&staker.0);
        }
        self.sum_stakes_now = sum;
        self.sum_stakes_all_time += sum;
    }
    fn set_latest_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(hash) = db.get(db::key(&db::Key::LatestBlockHash))? {
            self.latest_block = Block::get(db, &hash)?;
        }
        Ok(())
    }
    fn set_latest_block_hash(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: types::Hash,
    ) -> Result<(), Box<dyn Error>> {
        db.put(db::key(&db::Key::LatestBlockHash), hash)?;
        Ok(())
    }
    fn sort_pending_transactions(&mut self) {
        self.pending_transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
    }
    fn sort_pending_stakes(&mut self) {
        self.pending_stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
    }
    fn limit_pending_blocks(&mut self) {
        while self.pending_blocks.len() > PENDING_BLOCKS_LIMIT {
            self.pending_blocks.remove(self.pending_blocks.len() - 1);
        }
    }
    fn limit_pending_transactions(&mut self) {
        while self.pending_transactions.len() > PENDING_TRANSACTIONS_LIMIT {
            self.pending_transactions
                .remove(self.pending_transactions.len() - 1);
        }
    }
    fn limit_pending_stakes(&mut self) {
        while self.pending_stakes.len() > PENDING_STAKES_LIMIT {
            self.pending_stakes.remove(self.pending_stakes.len() - 1);
        }
    }
}
