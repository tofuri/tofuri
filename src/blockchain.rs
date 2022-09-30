use crate::{
    block::Block,
    constants::{
        BLOCK_STAKES_LIMIT, BLOCK_TRANSACTIONS_LIMIT, PENDING_BLOCKS_LIMIT, PENDING_STAKES_LIMIT,
        PENDING_TRANSACTIONS_LIMIT,
    },
    stake::Stake,
    states::States,
    transaction::Transaction,
    tree::Tree,
    types,
};
use colored::*;
use log::{info, trace};
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
    sync_iteration: usize,
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
            sync_iteration: 0,
            heartbeats: 0,
            lag: 0.0,
        }
    }
    pub fn get_sync_index(&self) -> &usize {
        &self.sync_index
    }
    pub fn get_sync_index_mut(&mut self) -> &mut usize {
        &mut self.sync_index
    }
    pub fn get_sync_iteration(&self) -> &usize {
        &self.sync_iteration
    }
    pub fn get_sync_iteration_mut(&mut self) -> &mut usize {
        &mut self.sync_iteration
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
        let hashes = self.states.get_current().get_hashes();
        if self.sync_index >= hashes.len() {
            self.sync_index = 0;
            self.sync_iteration += 1;
        }
        let hash = hashes[self.sync_index];
        trace!(
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
        let m_0 = if block.previous_hash == [0; 32] {
            [0; 32]
        } else {
            self.tree.main().unwrap().0
        };
        let new_branch = self
            .tree
            .insert(hash, block.previous_hash, block.timestamp)
            .unwrap();
        let m_1 = self.tree.main().unwrap().0;
        self.tree.sort_branches();
        let m_2 = self.tree.main().unwrap().0;
        if m_0 == block.previous_hash {
            if m_1 == m_2 {
                self.states.append(&self.db, block);
            } else if new_branch {
                self.reload();
            }
        }
        if let Some(index) = self.pending_blocks.iter().position(|x| x.hash() == hash) {
            self.pending_blocks.remove(index);
        }
        self.pending_transactions.clear();
        self.pending_stakes.clear();
        hash
    }
    pub fn reload(&mut self) {
        let start = Instant::now();
        self.tree.reload(&self.db);
        info!("{} {:?}", "Tree reload".cyan(), start.elapsed());
        if let Some(main) = self.tree.main() {
            info!(
                "{} {} {}",
                "Main branch".cyan(),
                main.1.to_string().yellow(),
                hex::encode(main.0)
            );
        }
        let start = Instant::now();
        self.states.reload(&self.db, self.tree.get_vec());
        info!("{} {:?}", "States reload".cyan(), start.elapsed());
    }
}
