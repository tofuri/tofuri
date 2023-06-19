use crate::b::BlockB;
use crate::Block;
use crate::Error;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_key::Key;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockA {
    pub hash: [u8; 32],
    pub previous_hash: [u8; 32],
    pub timestamp: u32,
    pub beta: [u8; 32],
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
    #[serde(with = "BigArray")]
    pub input_public_key: [u8; 33],
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    pub transactions: Vec<Transaction>,
    pub stakes: Vec<Stake>,
}
impl BlockA {
    pub fn b(&self) -> BlockB {
        BlockB {
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            signature: self.signature,
            pi: self.pi,
            transactions: self.transactions.clone(),
            stakes: self.stakes.clone(),
        }
    }
    pub fn sign(
        previous_hash: [u8; 32],
        timestamp: u32,
        transactions: Vec<Transaction>,
        stakes: Vec<Stake>,
        key: &Key,
        previous_beta: &[u8; 32],
    ) -> Result<BlockA, Error> {
        let pi = key.vrf_prove(previous_beta).map_err(Error::Key)?;
        let mut block_a = BlockA {
            hash: [0; 32],
            previous_hash,
            timestamp,
            beta: [0; 32],
            pi,
            input_public_key: [0; 33],
            signature: [0; 64],
            transactions,
            stakes,
        };
        block_a.beta = block_a.beta()?;
        block_a.hash = block_a.hash();
        block_a.signature = key.sign(&block_a.hash).map_err(Error::Key)?;
        block_a.input_public_key = key.public_key_bytes();
        Ok(block_a)
    }
    pub fn input_address(&self) -> [u8; 20] {
        Key::address(&self.input_public_key)
    }
    pub fn reward(&self) -> u128 {
        self.fees() + 10_u128.pow(18)
    }
    pub fn fees(&self) -> u128 {
        let mut fees = 0;
        for transaction in self.transactions.iter() {
            fees += transaction.fee;
        }
        for stake in self.stakes.iter() {
            fees += stake.fee;
        }
        fees
    }
    pub fn transaction_hashes(&self) -> Vec<[u8; 32]> {
        self.transactions.iter().map(|x| x.hash()).collect()
    }
    pub fn stake_hashes(&self) -> Vec<[u8; 32]> {
        self.stakes.iter().map(|x| x.hash()).collect()
    }
}
impl Block for BlockA {
    fn get_previous_hash(&self) -> &[u8; 32] {
        &self.previous_hash
    }
    fn get_merkle_root_transaction(&self) -> [u8; 32] {
        crate::merkle_root(&self.transaction_hashes())
    }
    fn get_merkle_root_stake(&self) -> [u8; 32] {
        crate::merkle_root(&self.stake_hashes())
    }
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_pi(&self) -> &[u8; 81] {
        &self.pi
    }
    fn hash(&self) -> [u8; 32] {
        crate::hash(self)
    }
    fn hash_input(&self) -> [u8; 181] {
        crate::hash_input(self)
    }
    fn beta(&self) -> Result<[u8; 32], Error> {
        crate::beta(self)
    }
}
impl Default for BlockA {
    fn default() -> BlockA {
        BlockA {
            hash: [0; 32],
            previous_hash: [0; 32],
            timestamp: 0,
            beta: [0; 32],
            pi: [0; 81],
            input_public_key: [0; 33],
            signature: [0; 64],
            transactions: vec![],
            stakes: vec![],
        }
    }
}
impl fmt::Debug for BlockA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockA")
            .field("hash", &hex::encode(self.hash))
            .field("previous_hash", &hex::encode(self.previous_hash))
            .field("timestamp", &self.timestamp)
            .field("beta", &hex::encode(self.beta))
            .field("pi", &hex::encode(self.pi))
            .field("input_public_key", &hex::encode(self.input_public_key))
            .field("signature", &hex::encode(self.signature))
            .field("transactions", &self.transactions)
            .field("stakes", &self.stakes)
            .finish()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::GENESIS_BLOCK_BETA;
    #[test]
    fn test_genesis_beta() {
        assert_eq!(BlockA::default().beta, GENESIS_BLOCK_BETA);
    }
}
