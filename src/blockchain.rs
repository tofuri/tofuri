use crate::{
    block::Block,
    constants::{
        BLOCK_STAKES_LIMIT, BLOCK_TIME_MAX, BLOCK_TRANSACTIONS_LIMIT, PENDING_BLOCKS_LIMIT,
        PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT,
    },
    stake::Stake,
    states::States,
    transaction::Transaction,
    tree::Tree,
    types, util,
};
use colored::*;
use log::info;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::{error::Error, time::Instant};
#[derive(Debug)]
pub struct Blockchain {
    db: DBWithThreadMode<SingleThreaded>,
    keypair: types::Keypair,
    tree: Tree,
    states: States,
    pending_transactions: Vec<Transaction>,
    pending_stakes: Vec<Stake>,
    pending_blocks: Vec<Block>,
    sync_index: usize,
    heartbeats: types::Heartbeats,
    lag: [f64; 3],
}
impl Blockchain {
    pub fn new(db: DBWithThreadMode<SingleThreaded>, keypair: types::Keypair) -> Self {
        let mut blockchain = Self {
            db,
            keypair,
            tree: Tree::default(),
            states: States::new(),
            pending_transactions: vec![],
            pending_stakes: vec![],
            pending_blocks: vec![],
            sync_index: 0,
            heartbeats: 0,
            lag: [0.0; 3],
        };
        let start = Instant::now();
        blockchain.reload();
        info!("{} {:?}", "Reload blockchain".cyan(), start.elapsed());
        blockchain
    }
    pub fn get_states(&self) -> &States {
        &self.states
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
    pub fn get_heartbeats(&self) -> &types::Heartbeats {
        &self.heartbeats
    }
    pub fn get_heartbeats_mut(&mut self) -> &mut types::Heartbeats {
        &mut self.heartbeats
    }
    pub fn get_keypair(&self) -> &types::Keypair {
        &self.keypair
    }
    pub fn get_db(&self) -> &DBWithThreadMode<SingleThreaded> {
        &self.db
    }
    pub fn get_lag(&self) -> &[f64; 3] {
        &self.lag
    }
    pub fn get_tree(&self) -> &Tree {
        &self.tree
    }
    pub fn get_height(&self) -> types::Height {
        if let Some(main) = self.tree.main() {
            main.1
        } else {
            0
        }
    }
    pub fn get_next_sync_block(&mut self) -> Block {
        let hashes = self.states.get_current().get_hashes();
        if self.sync_index >= hashes.len() {
            self.sync_index = 0;
        }
        let hash = hashes[self.sync_index];
        info!(
            "{} {} {}",
            "Sync".cyan(),
            self.sync_index.to_string().yellow(),
            hex::encode(&hash)
        );
        let block = Block::get(&self.db, &hash).unwrap();
        self.sync_index += 1;
        block
    }
    pub fn set_cold_start_stake(&mut self, stake: Stake) {
        self.pending_stakes = vec![stake];
    }
    pub fn set_lag(&mut self, millis: f64) {
        self.lag.rotate_right(1);
        self.lag[0] = millis;
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
    pub fn height(&self, hash: types::Hash) -> Option<types::Height> {
        self.states
            .get_current()
            .get_hashes()
            .iter()
            .position(|&x| x == hash)
    }
    pub fn pending_blocks_push(&mut self, block: Block) -> Result<(), Box<dyn Error>> {
        if self
            .pending_blocks
            .iter()
            .any(|b| b.signature == block.signature)
        {
            return Err("block already pending".into());
        }
        block.validate(self)?;
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
        let balance = self
            .states
            .get_current()
            .get_balance(&transaction.public_key_input);
        transaction.validate(
            &self.db,
            balance,
            self.states.get_current().get_latest_block().timestamp,
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
        let balance = self.states.get_current().get_balance(&stake.public_key);
        let balance_staked = self
            .states
            .get_current()
            .get_balance_staked(&stake.public_key);
        stake.validate(
            &self.db,
            balance,
            balance_staked,
            self.states.get_current().get_latest_block().timestamp,
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
        let hash = self.append(&block);
        info!(
            "{} {} {}",
            "Forged".green(),
            self.get_height().to_string().yellow(),
            hex::encode(hash)
        );
        Ok(block)
    }
    pub fn append_handle(&mut self) {
        if util::timestamp()
            > self.states.get_current().get_latest_block().timestamp
                + BLOCK_TIME_MAX as types::Timestamp
        {
            self.states.get_current_mut().penalty();
        }
        for block in self.pending_blocks.clone() {
            let hash = self.append(&block);
            info!(
                "{} {} {}",
                "Accepted".green(),
                self.tree.height(&block.previous_hash).to_string().yellow(),
                hex::encode(hash)
            );
        }
    }
    pub fn append(&mut self, block: &Block) -> types::Hash {
        block.put(&self.db).unwrap();
        let hash = block.hash();
        if let Some(new_branch) = self.tree.insert(hash, block.previous_hash) {
            let previous_hash = self.tree.main().unwrap().0;
            self.tree.sort_branches();
            self.states.append(&self.db, block);
            if new_branch && previous_hash != self.tree.main().unwrap().0 {
                self.reload();
            } else {
                self.pending_blocks.clear();
                self.pending_transactions.clear();
                self.pending_stakes.clear();
            }
        }
        hash
    }
    pub fn reload(&mut self) {
        self.tree.reload(&self.db);
        if let Some(main) = self.tree.main() {
            info!(
                "{} {} {}",
                "Main branch".cyan(),
                main.1.to_string().yellow(),
                hex::encode(main.0)
            );
        }
        self.states.reload(&self.db, self.tree.get_vec());
    }
    // pub fn get_balances_at_hash(
    // &self,
    // db: &DBWithThreadMode<SingleThreaded>,
    // balance_public_keys: Vec<types::PublicKeyBytes>,
    // balance_staked_public_keys: Vec<types::PublicKeyBytes>,
    // previous_hash: types::Hash,
    // ) -> (
    // HashMap<types::PublicKeyBytes, types::Amount>,
    // HashMap<types::PublicKeyBytes, types::Amount>,
    // ) {
    // let mut balances = HashMap::new();
    // let mut balances_staked = HashMap::new();
    // for public_key in balance_public_keys.iter() {
    // balances.insert(*public_key, self.get_balance(public_key));
    // }
    // for public_key in balance_staked_public_keys.iter() {
    // balances.insert(*public_key, self.get_balance(public_key));
    // balances_staked.insert(*public_key, self.get_balance_staked(public_key));
    // }
    // if let Some(main) = self.tree.main() {
    // let mut hash = main.0;
    // loop {
    // if hash == previous_hash || hash == [0; 32] {
    // break;
    // }
    // let block = Block::get(db, &hash).unwrap();
    // if let Some(balance_staked) = balances_staked.get(&block.public_key) {
    // let mut balance = *balances.get(&block.public_key).unwrap();
    // balance -= block.reward(*balance_staked);
    // if let Some(stake) = block.stakes.first() {
    // if stake.fee == 0 {
    // balance -= MIN_STAKE;
    // }
    // }
    // balances.insert(block.public_key, balance);
    // }
    // for transaction in block.transactions.iter() {
    // for public_key in balance_public_keys.iter() {
    // if public_key == &transaction.public_key_input {
    // let mut balance = *balances.get(public_key).unwrap();
    // balance += transaction.amount + transaction.fee;
    // balances.insert(*public_key, balance);
    // }
    // if public_key == &transaction.public_key_output {
    // let mut balance = *balances.get(public_key).unwrap();
    // balance -= transaction.amount;
    // balances.insert(*public_key, balance);
    // }
    // }
    // }
    // for stake in block.stakes.iter() {
    // for public_key in balance_staked_public_keys.iter() {
    // if public_key == &stake.public_key {
    // let mut balance = *balances.get(public_key).unwrap();
    // let mut balance_staked = *balances_staked.get(public_key).unwrap();
    // if stake.deposit {
    // balance += stake.amount + stake.fee;
    // balance_staked -= stake.amount;
    // } else {
    // balance -= stake.amount - stake.fee;
    // balance_staked += stake.amount;
    // }
    // balances.insert(*public_key, balance);
    // balances_staked.insert(*public_key, balance_staked);
    // }
    // }
    // }
    // match self.tree.get(&hash) {
    // Some(previous_hash) => hash = *previous_hash,
    // None => panic!("broken chain"),
    // };
    // }
    // }
    // (balances, balances_staked)
    // }
}
