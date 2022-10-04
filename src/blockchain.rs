use crate::{
    block::Block,
    constants::{
        BLOCK_STAKES_LIMIT, BLOCK_TIME_MIN, BLOCK_TRANSACTIONS_LIMIT, PENDING_BLOCKS_LIMIT,
        PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT,
    },
    stake::Stake,
    states::States,
    transaction::Transaction,
    tree::Tree,
    types,
};
use colored::*;
use log::{debug, info};
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
    sync_new: usize,
    sync_history: [usize; BLOCK_TIME_MIN],
    syncing: bool,
    heartbeats: types::Heartbeats,
    lag: f64,
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
            sync_index: 0,
            sync_new: 0,
            sync_history: [0; BLOCK_TIME_MIN],
            syncing: true,
            heartbeats: 0,
            lag: 0.0,
        }
    }
    pub fn heartbeat_handle(&mut self) {
        self.sync_history.rotate_right(1);
        self.sync_history[0] = self.sync_new;
        self.sync_new = 0;
        let mut sum = 0;
        for x in self.sync_history {
            sum += x;
        }
        self.syncing = sum > 1;
    }
    pub fn get_sync_index(&self) -> &usize {
        &self.sync_index
    }
    pub fn get_sync_new(&self) -> &usize {
        &self.sync_new
    }
    pub fn get_sync_history(&self) -> &[usize; BLOCK_TIME_MIN] {
        &self.sync_history
    }
    pub fn get_syncing(&self) -> &bool {
        &self.syncing
    }
    pub fn get_sync_index_mut(&mut self) -> &mut usize {
        &mut self.sync_index
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
    pub fn get_lag(&self) -> &f64 {
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
        let hashes_trusted = self.states.trusted.get_hashes();
        let hashes_dynamic = self.states.dynamic.get_hashes();
        if self.sync_index >= hashes_trusted.len() + hashes_dynamic.len() {
            self.sync_index = 0;
        }
        let hash = if self.sync_index < hashes_trusted.len() {
            hashes_trusted[self.sync_index]
        } else {
            hashes_dynamic[self.sync_index - hashes_trusted.len()]
        };
        debug!(
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
        self.lag = millis;
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
            .dynamic
            .get_balance(&transaction.public_key_input);
        transaction.validate(
            &self.db,
            balance,
            self.states.dynamic.get_latest_block().timestamp,
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
        let balance = self.states.dynamic.get_balance(&stake.public_key);
        let balance_staked = self.states.dynamic.get_balance_staked(&stake.public_key);
        stake.validate(
            &self.db,
            balance,
            balance_staked,
            self.states.dynamic.get_latest_block().timestamp,
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
    pub fn append_handle(&mut self) {
        // if util::timestamp()
        // > self.states.get_current().get_latest_block().timestamp
        // + BLOCK_TIME_MAX as types::Timestamp
        // {
        // self.states.get_current_mut().penalty();
        // }
        for block in self.pending_blocks.clone() {
            let hash = self.append(&block);
            info!(
                "{} {} {}",
                "Accept".green(),
                self.tree.height(&block.previous_hash).to_string().yellow(),
                hex::encode(hash)
            );
        }
    }
    pub fn append(&mut self, block: &Block) -> types::Hash {
        block.put(&self.db).unwrap();
        let hash = block.hash();
        if self
            .tree
            .insert(hash, block.previous_hash, block.timestamp)
            .unwrap()
        {
            info!("{}", "Fork".cyan());
        }
        self.tree.sort_branches();
        self.states.update(&self.db, &self.tree.get_vec_dynamic());
        if let Some(index) = self.pending_blocks.iter().position(|x| x.hash() == hash) {
            self.pending_blocks.remove(index);
        }
        self.pending_transactions.clear();
        self.pending_stakes.clear();
        if block.hash() == self.states.dynamic.get_latest_block().hash() {
            self.sync_new += 1;
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
        let (trusted, dynamic) = self.tree.get_vec();
        self.states.trusted.load(&self.db, &trusted);
        self.states
            .dynamic
            .reload(&self.db, &dynamic, &self.states.trusted);
        info!("{} {:?}", "States load".cyan(), start.elapsed());
    }
}
