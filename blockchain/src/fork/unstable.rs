use super::Error;
use super::Fork;
use super::Stable;
use block::Block;
use rocksdb::DB;
use serde::Deserialize;
use serde::Serialize;
use stake::Stake;
use std::collections::HashMap;
use std::collections::VecDeque;
use transaction::Transaction;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Unstable {
    pub latest_block: Block,
    pub hashes: Vec<[u8; 32]>,
    pub stakers: VecDeque<[u8; 20]>,
    latest_blocks: Vec<Block>,
    map_balance: HashMap<[u8; 20], u128>,
    map_staked: HashMap<[u8; 20], u128>,
}
impl Unstable {
    pub fn from(db: &DB, hashes: &[[u8; 32]], stable: &Stable) -> Unstable {
        let mut unstable = Unstable {
            hashes: vec![],
            stakers: stable.stakers.clone(),
            map_balance: stable.get_map_balance().clone(),
            map_staked: stable.get_map_staked().clone(),
            latest_block: Block::default(),
            latest_blocks: stable.get_latest_blocks().clone(),
        };
        super::load(&mut unstable, db, hashes);
        unstable
    }
    pub fn check_overflow(
        &self,
        transactions: &Vec<Transaction>,
        stakes: &Vec<Stake>,
    ) -> Result<(), Error> {
        let mut map_balance: HashMap<[u8; 20], u128> = HashMap::new();
        let mut map_staked: HashMap<[u8; 20], u128> = HashMap::new();
        for transaction in transactions {
            let k = transaction.input_address().unwrap();
            let mut balance = if map_balance.contains_key(&k) {
                *map_balance.get(&k).unwrap()
            } else {
                self.balance(&k)
            };
            balance = balance
                .checked_sub(u128::from(transaction.amount + transaction.fee))
                .ok_or(Error::Overflow)?;
            map_balance.insert(k, balance);
        }
        for stake in stakes {
            let k = stake.input_address().unwrap();
            let mut balance = if map_balance.contains_key(&k) {
                *map_balance.get(&k).unwrap()
            } else {
                self.balance(&k)
            };
            let mut staked = if map_staked.contains_key(&k) {
                *map_staked.get(&k).unwrap()
            } else {
                self.staked(&k)
            };
            if stake.deposit {
                balance = balance
                    .checked_sub((stake.amount + stake.fee).into())
                    .ok_or(Error::Overflow)?;
            } else {
                balance = balance
                    .checked_sub(stake.fee.into())
                    .ok_or(Error::Overflow)?;
                staked = staked
                    .checked_sub(stake.amount.into())
                    .ok_or(Error::Overflow)?;
            }
            map_balance.insert(k, balance);
            map_staked.insert(k, staked);
        }
        Ok(())
    }
    pub fn transaction_in_chain(&self, transaction: &Transaction) -> bool {
        for block in self.latest_blocks.iter() {
            if block
                .transactions
                .iter()
                .any(|a| a.hash() == transaction.hash())
            {
                return true;
            }
        }
        false
    }
    pub fn stake_in_chain(&self, stake: &Stake) -> bool {
        for block in self.latest_blocks.iter() {
            if block.stakes.iter().any(|a| a.hash() == stake.hash()) {
                return true;
            }
        }
        false
    }
    pub fn balance(&self, address: &[u8; 20]) -> u128 {
        super::get_balance(self, address)
    }
    pub fn staked(&self, address: &[u8; 20]) -> u128 {
        super::get_staked(self, address)
    }
    pub fn next_staker(&self, timestamp: u32) -> Option<[u8; 20]> {
        super::next_staker(self, timestamp)
    }
    pub fn stakers_offline(&self, timestamp: u32, previous_timestamp: u32) -> Vec<[u8; 20]> {
        super::stakers_offline(self, timestamp, previous_timestamp)
    }
    pub fn stakers_n(&self, n: usize) -> Vec<[u8; 20]> {
        super::stakers_n(self, n).0
    }
}
impl Fork for Unstable {
    fn get_hashes_mut(&mut self) -> &mut Vec<[u8; 32]> {
        &mut self.hashes
    }
    fn get_stakers(&self) -> &VecDeque<[u8; 20]> {
        &self.stakers
    }
    fn get_stakers_mut(&mut self) -> &mut VecDeque<[u8; 20]> {
        &mut self.stakers
    }
    fn get_map_balance(&self) -> &HashMap<[u8; 20], u128> {
        &self.map_balance
    }
    fn get_map_balance_mut(&mut self) -> &mut HashMap<[u8; 20], u128> {
        &mut self.map_balance
    }
    fn get_map_staked(&self) -> &HashMap<[u8; 20], u128> {
        &self.map_staked
    }
    fn get_map_staked_mut(&mut self) -> &mut HashMap<[u8; 20], u128> {
        &mut self.map_staked
    }
    fn get_latest_block(&self) -> &Block {
        &self.latest_block
    }
    fn get_latest_block_mut(&mut self) -> &mut Block {
        &mut self.latest_block
    }
    fn get_latest_blocks(&self) -> &Vec<Block> {
        &self.latest_blocks
    }
    fn get_latest_blocks_mut(&mut self) -> &mut Vec<Block> {
        &mut self.latest_blocks
    }
    fn is_stable() -> bool {
        false
    }
    fn append_block(&mut self, block: &Block, previous_timestamp: u32, loading: bool) {
        super::append_block(self, block, previous_timestamp, loading)
    }
}
