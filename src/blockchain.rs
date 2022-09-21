use crate::{
    address,
    block::Block,
    cli::ValidatorArgs,
    constants::{
        BLOCK_STAKES_LIMIT, BLOCK_TIME_MAX, BLOCK_TRANSACTIONS_LIMIT, MIN_STAKE,
        PENDING_BLOCKS_LIMIT, PENDING_STAKES_LIMIT, PENDING_TRANSACTIONS_LIMIT,
    },
    db,
    stake::Stake,
    synchronizer::Synchronizer,
    transaction::Transaction,
    tree::Tree,
    types, util,
};
use colored::*;
use libp2p::Multiaddr;
use log::{error, info, warn};
use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    time::Instant,
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
    db: DBWithThreadMode<SingleThreaded>,
    keypair: types::Keypair,
    multiaddrs: Vec<Multiaddr>,
    synchronizer: Synchronizer,
    heartbeats: types::Heartbeats,
    lag: [f64; 3],
    sync_index: usize,
    tree: Tree,
}
impl Blockchain {
    pub fn new(
        keypair: types::Keypair,
        db: DBWithThreadMode<SingleThreaded>,
        known: Vec<Multiaddr>,
    ) -> Self {
        let mut multiaddrs = known;
        multiaddrs.append(&mut Self::multiaddrs(&db).unwrap());
        let mut blockchain = Self {
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
            db,
            keypair,
            multiaddrs,
            synchronizer: Synchronizer::new(),
            heartbeats: 0,
            lag: [0.0; 3],
            sync_index: 0,
            tree: Tree::default(),
        };
        let start = Instant::now();
        blockchain.reload();
        info!("{} {:?}", "Reload blockchain".cyan(), start.elapsed());
        blockchain
    }
    pub fn put_multiaddr(
        db: &DBWithThreadMode<SingleThreaded>,
        multiaddr: &Multiaddr,
        timestamp: types::Timestamp,
    ) {
        db.put_cf(db::peers(db), multiaddr, timestamp.to_le_bytes())
            .unwrap();
    }
    pub fn multiaddrs(
        db: &DBWithThreadMode<SingleThreaded>,
    ) -> Result<Vec<Multiaddr>, Box<dyn Error>> {
        let mut multiaddrs = vec![];
        for i in db.iterator_cf(db::peers(db), IteratorMode::Start) {
            multiaddrs.push(String::from_utf8(i?.0.to_vec())?.parse()?);
        }
        Ok(multiaddrs)
    }
    pub fn get_known(args: &ValidatorArgs) -> Result<Vec<Multiaddr>, Box<dyn Error>> {
        let lines = util::read_lines(&args.known)?;
        let mut known = vec![];
        for line in lines {
            match line.parse() {
                Ok(multiaddr) => {
                    known.push(multiaddr);
                }
                Err(err) => error!("{}", err),
            }
        }
        Ok(known)
    }
    pub fn height(&self, hash: types::Hash) -> Option<types::Height> {
        self.hashes.iter().position(|&x| x == hash)
    }
    fn penalty(&mut self) {
        let public_key = self.stakers[0].0;
        self.balance_staked.remove(&public_key);
        self.stakers.remove(0).unwrap();
        warn!("{} {}", "Burned".red(), address::encode(&public_key));
    }
    fn penalty_reload(
        &mut self,
        timestamp: &types::Timestamp,
        previous_timestamp: &types::Timestamp,
    ) {
        if timestamp == previous_timestamp {
            return;
        }
        let diff = timestamp - previous_timestamp - 1;
        for _ in 0..diff / BLOCK_TIME_MAX as u32 {
            if !self.stakers.is_empty() {
                self.penalty();
            }
        }
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
        warn!(
            "{} {} {}",
            "Minted".cyan(),
            MIN_STAKE.to_string().yellow(),
            address::encode(&block.public_key).green()
        );
    }
    pub fn pending_blocks_push(&mut self, block: Block) -> Result<(), Box<dyn Error>> {
        if self
            .pending_blocks
            .iter()
            .any(|b| b.signature == block.signature)
        {
            return Err("block already pending".into());
        }
        block.validate(self, &self.db)?;
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
        let balance = self.get_balance(&transaction.public_key_input);
        transaction.validate(&self.db, balance, self.latest_block.timestamp)?;
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
        let balance = self.get_balance(&stake.public_key);
        let balance_staked = self.get_balance_staked(&stake.public_key);
        stake.validate(
            &self.db,
            balance,
            balance_staked,
            self.latest_block.timestamp,
        )?;
        self.pending_stakes.push(stake);
        self.limit_pending_stakes();
        Ok(())
    }
    pub fn forge_block(&mut self) -> Result<Block, Box<dyn Error>> {
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
        block.sign(&self.keypair);
        let hash = self.append(block.clone(), true).unwrap();
        info!(
            "{} {} {}",
            "Forged".green(),
            self.get_height().to_string().yellow(),
            hex::encode(hash)
        );
        Ok(block)
    }
    pub fn append_handle(&mut self) {
        if util::timestamp() > self.latest_block.timestamp + BLOCK_TIME_MAX as types::Timestamp {
            self.penalty();
            warn!("staker didn't show up in time");
        }
        for block in self.pending_blocks.clone() {
            // if let Some(public_key) = self.stakers_history.get(&block.previous_hash) {
            // if public_key != &block.public_key {
            // warn!("block public_key don't match stakers history");
            // return;
            // }
            // } else {
            // warn!("block didn't have a staker because network was down");
            // }
            let hash = if block.previous_hash == self.latest_block.hash()
                || self.latest_block.previous_hash == [0; 32]
            {
                self.append(block, true).unwrap()
            } else {
                self.append(block, false).unwrap()
            };
            info!(
                "{} {} {}",
                "Accepted".green(),
                self.get_height().to_string().yellow(),
                hex::encode(hash)
            );
        }
    }
    pub fn append(&mut self, block: Block, latest: bool) -> Result<types::Hash, Box<dyn Error>> {
        block.put(&self.db)?;
        let hash = block.hash();
        if latest {
            // self.set_latest_block();
            self.hashes.push(hash);
            if let Some(stake) = block.stakes.first() {
                if stake.fee == 0 {
                    self.reward_cold_start(&block);
                }
            }
            self.reward(&block);
            self.set_balances(&block);
            self.set_stakers(self.get_height(), &block);
            self.set_sum_stakes();
            self.latest_block = block;
            self.pending_blocks.clear();
            self.pending_transactions.clear();
            self.pending_stakes.clear();
        } else if let Some(height) = self.height(block.previous_hash) {
            if height + 1 > self.get_height() {
                warn!("Fork detected! Reloading...");
                self.reload();
            }
        }
        Ok(hash)
    }
    pub fn reload(&mut self) {
        self.latest_block = Block::new([0; 32]);
        self.stakers.clear();
        self.hashes.clear();
        self.balance.clear();
        self.balance_staked.clear();
        self.stakers_history.clear();
        self.tree.reload(&self.db);
        if let Some(main) = self.tree.main() {
            info!(
                "{} {} {}",
                "Main branch".cyan(),
                main.1.to_string().yellow(),
                hex::encode(main.0)
            );
        }
        self.set_latest_block();
        let hashes = self.tree.get_main_hashes();
        let mut previous_block_timestamp = match hashes.first() {
            Some(hash) => Block::get(&self.db, hash).unwrap().timestamp - 1,
            None => 0,
        };
        for (height, hash) in hashes.iter().enumerate() {
            let block = Block::get(&self.db, hash).unwrap();
            self.penalty_reload(&block.timestamp, &previous_block_timestamp);
            if let Some(stake) = block.stakes.first() {
                if stake.fee == 0 {
                    self.reward_cold_start(&block);
                }
            }
            self.reward(&block);
            self.set_balances(&block);
            self.set_stakers(height, &block);
            self.set_sum_stakes();
            previous_block_timestamp = block.timestamp;
        }
        self.hashes = hashes;
        self.penalty_reload(&util::timestamp(), &self.latest_block.timestamp.clone());
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
    pub fn get_balances_at_height(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        balance_public_keys: Vec<types::PublicKeyBytes>,
        balance_staked_public_keys: Vec<types::PublicKeyBytes>,
        height: types::Height,
    ) -> (
        HashMap<types::PublicKeyBytes, types::Amount>,
        HashMap<types::PublicKeyBytes, types::Amount>,
    ) {
        let mut balances = HashMap::new();
        let mut balances_staked = HashMap::new();
        for public_key in balance_public_keys.iter() {
            balances.insert(*public_key, self.get_balance(public_key));
        }
        for public_key in balance_staked_public_keys.iter() {
            balances.insert(*public_key, self.get_balance(public_key));
            balances_staked.insert(*public_key, self.get_balance_staked(public_key));
        }
        let n = self.get_height() - height;
        for hash in self.hashes.iter().rev().take(n) {
            let block = Block::get(db, hash).unwrap();
            for transaction in block.transactions.iter() {
                for public_key in balance_public_keys.iter() {
                    if public_key == &transaction.public_key_input {
                        let mut balance = *balances.get(public_key).unwrap();
                        balance += transaction.amount + transaction.fee;
                        balances.insert(*public_key, balance);
                    }
                    if public_key == &transaction.public_key_output {
                        let mut balance = *balances.get(public_key).unwrap();
                        balance -= transaction.amount;
                        balances.insert(*public_key, balance);
                    }
                }
            }
            for stake in block.stakes.iter() {
                for public_key in balance_staked_public_keys.iter() {
                    if public_key == &stake.public_key {
                        let mut balance = *balances.get(public_key).unwrap();
                        let mut balance_staked = *balances_staked.get(public_key).unwrap();
                        if stake.deposit {
                            balance += stake.amount + stake.fee;
                            balance_staked -= stake.amount;
                        } else {
                            balance -= stake.amount - stake.fee;
                            balance_staked += stake.amount;
                        }
                        balances.insert(*public_key, balance);
                        balances_staked.insert(*public_key, balance_staked);
                    }
                }
            }
        }
        (balances, balances_staked)
    }
    pub fn get_multiaddrs(&self) -> &Vec<Multiaddr> {
        &self.multiaddrs
    }
    pub fn get_heartbeats(&self) -> &types::Heartbeats {
        &self.heartbeats
    }
    pub fn get_heartbeats_mut(&mut self) -> &mut types::Heartbeats {
        &mut self.heartbeats
    }
    pub fn get_synchronizer(&self) -> &Synchronizer {
        &self.synchronizer
    }
    pub fn get_synchronizer_mut(&mut self) -> &mut Synchronizer {
        &mut self.synchronizer
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
    pub fn get_next_sync_block(&mut self) -> Block {
        if self.sync_index >= self.hashes.len() {
            self.sync_index = 0;
        }
        let block = Block::get(&self.db, &self.hashes[self.sync_index]).unwrap();
        self.sync_index += 1;
        block
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
                warn!(
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
    fn set_latest_block(&mut self) {
        self.latest_block = if let Some(main) = self.tree.main() {
            let hash = main.0;
            Block::get(&self.db, &hash).unwrap()
        } else {
            Block::new([0; 32])
        };
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
}
