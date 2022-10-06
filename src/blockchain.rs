use crate::{
    block::Block,
    constants::{
        BLOCK_STAKES_LIMIT, BLOCK_TRANSACTIONS_LIMIT, PENDING_BLOCKS_LIMIT, PENDING_STAKES_LIMIT,
        PENDING_TRANSACTIONS_LIMIT,
    },
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
        let block = Block::get(&self.db, &hash).unwrap();
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
        let balance = self.states.dynamic.balance(&transaction.public_key_input);
        transaction.validate(
            &self.db,
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
        stake.validate(
            &self.db,
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
}
