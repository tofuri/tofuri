use crate::{
    address,
    block::{Block, BlockMetadata},
    constants::{
        BLOCK_STAKES_LIMIT,
        BLOCK_TIME_MAX,
        BLOCK_TIME_MIN,
        BLOCK_TRANSACTIONS_LIMIT,
        DECIMAL_PRECISION,
        GENESIS_TIMESTAMP,
        MAX_STAKE, // BLOCKS_BEFORE_UNSTAKE
        MIN_STAKE,
        MIN_STAKE_MULTIPLIER,
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
use std::error::Error;
#[derive(Debug)]
pub struct Blockchain {
    pub latest_block: Block,
    pub hashes: types::Hashes,
    pub stakers: types::Stakers,
    pub pending_transactions: Vec<Transaction>,
    pub pending_stakes: Vec<Stake>,
    pub pending_blocks: Vec<Block>,
}
impl Blockchain {
    pub fn new(db: &DBWithThreadMode<SingleThreaded>) -> Result<Blockchain, Box<dyn Error>> {
        let latest_block = Blockchain::get_latest_block(db)?;
        let hashes = Blockchain::hashes(db, BlockMetadata::from(&latest_block).hash())?;
        // reinitialize latest_block in case of a broken chain
        let latest_block = Blockchain::get_latest_block(db)?;
        let stakers = Blockchain::stakers(db, &hashes)?;
        Ok(Blockchain {
            latest_block,
            hashes,
            stakers,
            pending_transactions: vec![],
            pending_stakes: vec![],
            pending_blocks: vec![],
        })
    }
    pub fn stakers(
        db: &DBWithThreadMode<SingleThreaded>,
        hashes: &[types::Hash],
    ) -> Result<types::Stakers, Box<dyn Error>> {
        let mut stakers = types::Stakers::new();
        for (index, hash) in hashes.iter().enumerate() {
            let block = Block::get(db, hash)?;
            for stake in block.stakes {
                // check if stake index already exists for public_key before pushing
                if !stakers.iter().any(|&e| e.0 == stake.public_key) {
                    stakers.push_back((stake.public_key, index));
                }
            }
        }
        Ok(stakers)
    }
    pub fn forge_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        keypair: &types::Keypair,
    ) -> Result<Block, Box<dyn Error>> {
        let latest_block_metadata = BlockMetadata::from(&self.latest_block);
        let mut block = Block::new(latest_block_metadata.hash());
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
        let mut block_metadata = BlockMetadata::from(&block);
        block_metadata.sign(keypair);
        block.public_key = block_metadata.public_key;
        block.signature = block_metadata.signature;
        self.try_add_block(db, block.clone())?;
        info!(
            "{}: {} @ {}",
            "Forged".cyan(),
            (self.latest_height() + 1).to_string().yellow(),
            hex::encode(block_metadata.hash()).green()
        );
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
        if !self.stakers.is_empty() {
            if block.timestamp < self.latest_block.timestamp + BLOCK_TIME_MIN as types::Timestamp {
                return Err("block created too early".into());
            }
            if block.timestamp > self.latest_block.timestamp + BLOCK_TIME_MAX as types::Timestamp {
                return Err("block created too late".into());
            }
        }
        let previous_block = Block::get(db, &block.previous_hash)?;
        if self.latest_block.previous_hash != previous_block.previous_hash {
            return Err("block does not extend active chain".into());
        }
        // TRANSACTIONS TRANSACTIONS TRANSACTIONS TRANSACTIONS TRANSACTIONS TRANSACTIONS
        for transaction in block.transactions.iter() {
            self.validate_transaction(db, transaction)?;
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
        if self.stakers.is_empty() {
            self.validate_mint_stake(&block.stakes)?;
        } else {
            for stake in block.stakes.iter() {
                self.validate_stake(db, stake)?;
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
    ) -> Result<(), Box<dyn Error>> {
        if !transaction.is_valid() {
            return Err("transaction not valid".into());
        }
        if Transaction::get(db, &transaction.hash()).is_ok() {
            return Err("transaction already in chain".into());
        }
        let balance = self.get_balance(db, &transaction.public_key_input)?;
        if transaction.amount + transaction.fee > balance {
            return Err("transaction too expensive".into());
        }
        if transaction.timestamp < self.latest_block.timestamp {
            return Err("transaction too old (created before latest block)".into());
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
        self.validate_transaction(db, &transaction)?;
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
    fn validate_mint_stake(&self, stakes: &Vec<Stake>) -> Result<(), Box<dyn Error>> {
        if stakes.len() != 1 {
            return Err("only allowed to mind 1 stake".into());
        }
        let stake = stakes.get(0).unwrap();
        if !stake.is_valid() {
            return Err("stake not valid".into());
        }
        if stake.timestamp < self.latest_block.timestamp {
            return Err("stake too old (created before latest block)".into());
        }
        if !stake.deposit {
            return Err("mint stake must be deposit".into());
        }
        if stake.amount != MIN_STAKE {
            return Err("stake invalid amount".into());
        }
        if stake.fee != 0 {
            return Err("stake invalid fee".into());
        }
        Ok(())
    }
    fn validate_stake(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        stake: &Stake,
    ) -> Result<(), Box<dyn Error>> {
        if !stake.is_valid() {
            return Err("stake not valid".into());
        }
        if Stake::get(db, &stake.hash()).is_ok() {
            return Err("stake already in chain".into());
        }
        let balance = self.get_balance(db, &stake.public_key)?;
        let staked_balance = self.get_staked_balance(db, &stake.public_key)?;
        if stake.deposit {
            if stake.amount + stake.fee > balance {
                return Err("stake deposit too expensive".into());
            }
            if stake.amount + staked_balance > MAX_STAKE {
                return Err("stake deposit exceeds MAX_STAKE".into());
            }
        } else {
            if stake.fee > balance {
                return Err("stake withdraw insufficient funds".into());
            }
            if stake.amount > staked_balance {
                return Err("stake withdraw too expensive".into());
            }
        }
        if stake.timestamp < self.latest_block.timestamp {
            return Err("stake too old (created before latest block)".into());
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
        self.validate_stake(db, &stake)?;
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
    pub fn height(&self, hash: types::Hash) -> Option<types::Height> {
        self.hashes.iter().position(|&x| x == hash)
    }
    pub fn latest_height(&self) -> types::Height {
        self.hashes.len() - 1
    }
    pub fn sum_stakes(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
    ) -> Result<types::Amount, Box<dyn Error>> {
        let mut sum = 0;
        for staker in self.stakers.iter() {
            sum += self.get_staked_balance(db, &staker.0)?;
        }
        Ok(sum)
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
                    let block_metadata = BlockMetadata::from(&block);
                    let hash = block_metadata.hash();
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
    fn genesis_block() -> Block {
        Block::from([0; 32], GENESIS_TIMESTAMP, [0; 32], [0; 64])
    }
    fn get_latest_block(db: &DBWithThreadMode<SingleThreaded>) -> Result<Block, Box<dyn Error>> {
        let bytes = db.get(db::key(&db::Key::LatestBlockHash))?;
        if let Some(bytes) = bytes {
            // latest_block is set
            Block::get(db, &bytes)
        } else {
            // latest_block is NOT set
            // should be the case if the blockchain haven't been initialized
            let block = Blockchain::genesis_block();
            block.put(db)?;
            let block_metadata = BlockMetadata::from(&block);
            Blockchain::put_latest_block_hash(db, block_metadata.hash())?;
            Ok(block)
        }
    }
    fn put_latest_block_hash(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: types::Hash,
    ) -> Result<(), Box<dyn Error>> {
        db.put(db::key(&db::Key::LatestBlockHash), hash)?;
        Ok(())
    }
    fn get_balance_raw(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &types::PublicKeyBytes,
    ) -> Result<types::Amount, Box<dyn Error>> {
        let bytes = db
            .get_cf(db::cf_handle_balances(db)?, public_key)?
            .ok_or("balance not found")?;
        Ok(types::Amount::from_le_bytes(bytes.as_slice().try_into()?))
    }
    pub fn get_balance(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &types::PublicKeyBytes,
    ) -> Result<types::Amount, Box<dyn Error>> {
        match self.get_balance_raw(db, public_key) {
            Ok(balance) => Ok(balance),
            Err(err) => {
                if err.to_string() == "balance not found" {
                    Ok(0)
                } else {
                    Err(err)
                }
            }
        }
    }
    fn put_balance(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &types::PublicKeyBytes,
        balance: types::Amount,
    ) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::cf_handle_balances(db)?,
            public_key,
            balance.to_le_bytes(),
        )?;
        Ok(())
    }
    fn get_staked_balance_raw(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &types::PublicKeyBytes,
    ) -> Result<types::Amount, Box<dyn Error>> {
        let bytes = db
            .get_cf(db::cf_handle_staked_balances(db)?, public_key)?
            .ok_or("staked_balance not found")?;
        Ok(types::Amount::from_le_bytes(bytes.as_slice().try_into()?))
    }
    pub fn get_staked_balance(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &types::PublicKeyBytes,
    ) -> Result<types::Amount, Box<dyn Error>> {
        match self.get_staked_balance_raw(db, public_key) {
            Ok(balance) => Ok(balance),
            Err(err) => {
                if err.to_string() == "staked_balance not found" {
                    Ok(0)
                } else {
                    Err(err)
                }
            }
        }
    }
    fn put_staked_balance(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &types::PublicKeyBytes,
        staked_balance: types::Amount,
    ) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::cf_handle_staked_balances(db)?,
            public_key,
            staked_balance.to_le_bytes(),
        )?;
        Ok(())
    }
    fn add_reward(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: &types::PublicKeyBytes,
        fees: types::Amount,
    ) -> Result<(), Box<dyn Error>> {
        let staked_balance = self.get_staked_balance(db, public_key)?;
        let mut balance = self.get_balance(db, public_key)?;
        balance += Blockchain::reward(staked_balance);
        balance += fees;
        self.put_balance(db, public_key, balance)?;
        Ok(())
    }
    fn cache_balances(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        transactions: &Vec<Transaction>,
        stakes: &Vec<Stake>,
    ) -> Result<(), Box<dyn Error>> {
        for t in transactions {
            // input
            let mut balance = self.get_balance(db, &t.public_key_input)?;
            balance -= t.amount + t.fee;
            self.put_balance(db, &t.public_key_input, balance)?;
            // output
            let mut balance = self.get_balance(db, &t.public_key_output)?;
            balance += t.amount;
            self.put_balance(db, &t.public_key_output, balance)?;
        }
        for s in stakes {
            if s.deposit {
                // input
                let mut balance = self.get_balance(db, &s.public_key)?;
                balance -= s.amount + s.fee;
                self.put_balance(db, &s.public_key, balance)?;
                let mut staked_balance = self.get_staked_balance(db, &s.public_key)?;
                staked_balance += s.amount;
                self.put_staked_balance(db, &s.public_key, staked_balance)?;
            } else {
                // output
                let mut balance = self.get_balance(db, &s.public_key)?;
                balance += s.amount;
                balance -= s.fee;
                self.put_balance(db, &s.public_key, balance)?;
                let mut staked_balance = self.get_staked_balance(db, &s.public_key)?;
                staked_balance -= s.amount;
                self.put_staked_balance(db, &s.public_key, staked_balance)?;
            }
        }
        Ok(())
    }
    fn get_fees(transactions: &Vec<Transaction>, stakes: &Vec<Stake>) -> types::Amount {
        let mut fees = 0;
        for t in transactions {
            fees += t.fee;
        }
        for s in stakes {
            fees += s.fee;
        }
        fees
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
    pub fn reward(stake_amount: types::Amount) -> types::Amount {
        ((2f64
            .powf((stake_amount as f64 / DECIMAL_PRECISION as f64) / MIN_STAKE_MULTIPLIER as f64)
            - 1f64)
            * DECIMAL_PRECISION as f64) as types::Amount
    }
    fn next_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
    ) -> Result<Block, Box<dyn Error>> {
        if self.pending_blocks.is_empty()
            && !self.stakers.is_empty()
            && util::timestamp() > self.latest_block.timestamp + BLOCK_TIME_MAX as types::Timestamp
        {
            self.punish_staker_first_in_queue(db)?;
            return Err("validator did not show up 1".into());
        }
        if self.pending_blocks.is_empty() {
            return Err("no pending blocks".into());
        }
        let block;
        if self.stakers.is_empty() {
            block = self.pending_blocks.remove(0);
            self.cold_start_mint_stakers_stakes(db, &block)?;
        } else if let Some(index) = self
            .pending_blocks
            .iter()
            .position(|x| x.public_key == self.stakers[0].0)
        {
            block = self.pending_blocks.remove(index)
        } else {
            self.punish_staker_first_in_queue(db)?;
            return Err("validator did not show up 2".into());
        }
        Ok(block)
    }
    pub fn accept_block(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        forger: bool,
    ) -> Result<(), Box<dyn Error>> {
        let block = self.next_block(db)?;
        block.put(db)?;
        let block_metadata = BlockMetadata::from(&block);
        let hash = block_metadata.hash();
        self.hashes.push(hash);
        Blockchain::put_latest_block_hash(db, hash)?;
        self.cache_balances(db, &block.transactions, &block.stakes)?;
        let fees = Blockchain::get_fees(&block.transactions, &block.stakes);
        self.add_reward(db, &block.public_key, fees)?;
        if self.stakers.len() > 1 {
            self.stakers.rotate_left(1);
        }
        for stake in block.stakes.iter() {
            let staked_balance = self.get_staked_balance(db, &stake.public_key)?;
            if stake.deposit {
                if staked_balance >= MIN_STAKE
                    && !self.stakers.iter().any(|&e| e.0 == stake.public_key)
                {
                    self.stakers
                        .push_back((stake.public_key, self.latest_height()));
                }
            } else if staked_balance < MIN_STAKE {
                self.put_staked_balance(db, &stake.public_key, 0)?; // burn low "staked balance" to make sure "staked balance" never exceeds MAX_STAKE after being minted
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
        self.latest_block = block;
        self.pending_blocks.clear();
        if !forger {
            info!(
                "{}: {} {}",
                "Accepted".green(),
                self.latest_height().to_string().yellow(),
                hex::encode(hash)
            );
        }
        Ok(())
    }
    fn punish_staker_first_in_queue(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
    ) -> Result<(), Box<dyn Error>> {
        let staker = self.stakers[0];
        let public_key = staker.0;
        self.put_staked_balance(db, &public_key, 0)?;
        self.stakers.remove(0).unwrap();
        log::warn!("{}: {}", "Burned".red(), address::encode(&public_key));
        Ok(())
    }
    fn cold_start_mint_stakers_stakes(
        &mut self,
        db: &DBWithThreadMode<SingleThreaded>,
        block: &Block,
    ) -> Result<(), Box<dyn Error>> {
        log::warn!(
            "{}",
            "Staker queue should not be empty unless the network just started up.".yellow()
        );
        for stake in block.stakes.iter() {
            let mut balance = self.get_balance(db, &stake.public_key)?;
            let minted = stake.amount + stake.fee;
            balance += minted;
            self.put_balance(db, &stake.public_key, balance)?;
            log::warn!(
                "{}: {} @ {}",
                "Minted".cyan(),
                minted.to_string().yellow(),
                address::encode(&stake.public_key).green()
            );
        }
        Ok(())
    }
}
