use crate::{
    block::Block,
    constants::{
        BLOCK_STAKES_LIMIT, BLOCK_TIME_MIN, BLOCK_TRANSACTIONS_LIMIT, MAX_STAKE, MIN_STAKE,
        PENDING_BLOCKS_LIMIT, PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT,
    },
    db,
    stake::Stake,
    state::Dynamic,
    states::States,
    sync::Sync,
    transaction::Transaction,
    tree::Tree,
    types,
};
use colored::*;
use log::{debug, info};
use pea_core::util;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::{error::Error, time::Instant};
#[derive(Debug)]
pub struct Blockchain {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub keypair: types::Keypair,
    pub tree: Tree,
    pub states: States,
    pub pending_transactions: Vec<Transaction>,
    pub pending_stakes: Vec<Stake>,
    pub pending_blocks: Vec<Block>,
    pub sync: Sync,
}
impl Blockchain {
    pub fn new(db: DBWithThreadMode<SingleThreaded>, keypair: types::Keypair) -> Self {
        Self {
            db,
            keypair,
            tree: Tree::default(),
            states: States::default(),
            pending_transactions: vec![],
            pending_stakes: vec![],
            pending_blocks: vec![],
            sync: Sync::default(),
        }
    }
    pub fn height(&self) -> types::Height {
        if let Some(main) = self.tree.main() {
            main.1
        } else {
            0
        }
    }
    pub fn sync_block(&mut self) -> Block {
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
        debug!(
            "{} {} {}",
            "Sync".cyan(),
            self.sync.index.to_string().yellow(),
            hex::encode(&hash)
        );
        let block = db::block::get(&self.db, &hash).unwrap();
        self.sync.index += 1;
        block
    }
    pub fn set_cold_start_stake(&mut self, stake: Stake) {
        self.pending_stakes = vec![stake];
    }
    fn sort_pending_transactions(&mut self) {
        self.pending_transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
    }
    fn sort_pending_stakes(&mut self) {
        self.pending_stakes.sort_by(|a, b| b.fee.cmp(&a.fee));
    }
    fn limit_pending_blocks(&mut self) {
        while self.pending_blocks.len() > PENDING_BLOCKS_LIMIT {
            self.pending_blocks.remove(0);
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
    pub fn pending_blocks_push(&mut self, block: Block) -> Result<(), Box<dyn Error>> {
        if self
            .pending_blocks
            .iter()
            .any(|b| b.signature == block.signature)
        {
            return Err("block already pending".into());
        }
        self.validate_block(&block)?;
        self.pending_blocks.push(block);
        self.limit_pending_blocks();
        Ok(())
    }
    pub fn pending_transactions_push(
        &mut self,
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
        let balance = self.states.dynamic.balance(&transaction.public_key_input);
        self.validate_transaction(
            &transaction,
            balance,
            self.states.dynamic.latest_block.timestamp,
        )?;
        self.pending_transactions.push(transaction);
        self.limit_pending_transactions();
        Ok(())
    }
    pub fn pending_stakes_push(&mut self, stake: Stake) -> Result<(), Box<dyn Error>> {
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
        let balance = self.states.dynamic.balance(&stake.public_key);
        let balance_staked = self.states.dynamic.balance_staked(&stake.public_key);
        self.validate_stake(
            &stake,
            balance,
            balance_staked,
            self.states.dynamic.latest_block.timestamp,
        )?;
        self.pending_stakes.push(stake);
        self.limit_pending_stakes();
        Ok(())
    }
    pub fn forge_block(&mut self) -> Result<Block, Box<dyn Error>> {
        let mut block;
        if let Some(main) = self.tree.main() {
            block = Block::new(main.0);
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
        block.sign(&self.keypair);
        self.pending_blocks_push(block.clone())?;
        info!(
            "{} {} {}",
            "Forged".magenta(),
            self.tree.height(&block.previous_hash).to_string().yellow(),
            hex::encode(block.hash())
        );
        Ok(block)
    }
    pub fn pending_blocks_accept(&mut self) {
        for block in self.pending_blocks.clone() {
            let hash = self.block_accept(&block);
            info!(
                "{} {} {}",
                "Accept".green(),
                self.tree.height(&block.previous_hash).to_string().yellow(),
                hex::encode(hash)
            );
        }
    }
    pub fn block_accept(&mut self, block: &Block) -> types::Hash {
        db::block::put(&block, &self.db).unwrap();
        let hash = block.hash();
        if self
            .tree
            .insert(hash, block.previous_hash, block.timestamp)
            .unwrap()
        {
            info!("{}", "Fork".cyan());
        }
        self.tree.sort_branches();
        self.states.update(&self.db, &self.tree.hashes_dynamic());
        if let Some(index) = self.pending_blocks.iter().position(|x| x.hash() == hash) {
            self.pending_blocks.remove(index);
        }
        self.pending_transactions.clear();
        self.pending_stakes.clear();
        if block.hash() == self.states.dynamic.latest_block.hash() {
            self.sync.new += 1;
        }
        hash
    }
    pub fn load(&mut self) {
        let start = Instant::now();
        self.tree.reload(&self.db);
        info!("{} {:?}", "Tree load".cyan(), start.elapsed());
        if let Some(main) = self.tree.main() {
            info!(
                "{} {} {}",
                "Main branch".cyan(),
                main.1.to_string().yellow(),
                hex::encode(main.0)
            );
        }
        let start = Instant::now();
        let (hashes_trusted, hashes_dynamic) = self.tree.hashes();
        self.states.trusted.load(&self.db, &hashes_trusted);
        self.states.dynamic = Dynamic::from(&self.db, &hashes_dynamic, &self.states.trusted);
        info!("{} {:?}", "States load".cyan(), start.elapsed());
    }
    pub fn validate_block(&self, block: &Block) -> Result<(), Box<dyn Error>> {
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
        let public_key_inputs = block
            .transactions
            .iter()
            .map(|t| t.public_key_input)
            .collect::<Vec<types::PublicKeyBytes>>();
        if (1..public_key_inputs.len())
            .any(|i| public_key_inputs[i..].contains(&public_key_inputs[i - 1]))
        {
            return Err("block includes multiple transactions from same public_key_input".into());
        }
        let public_keys = block
            .stakes
            .iter()
            .map(|s| s.public_key)
            .collect::<Vec<types::PublicKeyBytes>>();
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
                    return Err(
                        "mint stake has invalid timestamp (mint stake is from the future)".into(),
                    );
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
    pub fn validate_transaction(
        &self,
        transaction: &Transaction,
        balance: types::Amount,
        timestamp: types::Timestamp,
    ) -> Result<(), Box<dyn Error>> {
        if types::PublicKey::from_bytes(&transaction.public_key_output).is_err() {
            return Err("transaction has invalid public_key_output".into());
        }
        if transaction.verify().is_err() {
            return Err("transaction has invalid signature".into());
        }
        if transaction.timestamp > util::timestamp() {
            return Err(
                "transaction has invalid timestamp (transaction is from the future)".into(),
            );
        }
        if transaction.public_key_input == transaction.public_key_output {
            return Err("transaction public_key_input == public_key_output".into());
        }
        if transaction.amount == 0 {
            return Err("transaction has invalid amount".into());
        }
        if transaction.fee == 0 {
            return Err("transaction invalid fee".into());
        }
        if db::transaction::get(&self.db, &transaction.hash()).is_ok() {
            return Err("transaction already in chain".into());
        }
        if transaction.amount + transaction.fee > balance {
            return Err("transaction too expensive".into());
        }
        if transaction.timestamp < timestamp {
            return Err("transaction too old".into());
        }
        Ok(())
    }
    pub fn validate_stake(
        &self,
        stake: &Stake,
        balance: types::Amount,
        balance_staked: types::Amount,
        timestamp: types::Timestamp,
    ) -> Result<(), Box<dyn Error>> {
        if stake.verify().is_err() {
            return Err("stake has invalid signature".into());
        }
        if stake.amount == 0 {
            return Err("stake has invalid amount".into());
        }
        if stake.fee == 0 {
            return Err("stake invalid fee".into());
        }
        if stake.timestamp > util::timestamp() {
            return Err("stake has invalid timestamp (stake is from the future)".into());
        }
        if db::stake::get(&self.db, &stake.hash()).is_ok() {
            return Err("stake already in chain".into());
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
}
