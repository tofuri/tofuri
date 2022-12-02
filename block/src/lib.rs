use pea_core::{types, util};
use pea_key::Key;
use pea_stake::Stake;
use pea_transaction::Transaction;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub previous_hash: types::Hash,
    pub transaction_merkle_root: types::MerkleRoot,
    pub stake_merkle_root: types::MerkleRoot,
    pub timestamp: u32,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Block {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    pub public_key: types::PublicKeyBytes,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    pub transactions: Vec<Transaction>,
    pub stakes: Vec<Stake>,
}
impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #![allow(dead_code)]
        #[derive(Debug)]
        struct Block {
            hash: String,
            previous_hash: String,
            timestamp: u32,
            public_key: String,
            signature: String,
            transactions: Vec<String>,
            stakes: Vec<String>,
        }
        write!(
            f,
            "{:?}",
            Block {
                hash: hex::encode(self.hash()),
                previous_hash: hex::encode(self.previous_hash),
                timestamp: self.timestamp,
                public_key: pea_address::public::encode(&self.public_key),
                signature: hex::encode(self.signature),
                transactions: self.transactions.iter().map(|x| hex::encode(x.hash())).collect(),
                stakes: self.stakes.iter().map(|x| hex::encode(x.hash())).collect(),
            }
        )
    }
}
impl Block {
    pub fn new(previous_hash: types::Hash, timestamp: u32) -> Block {
        Block {
            previous_hash,
            timestamp,
            public_key: [0; 32],
            signature: [0; 64],
            transactions: vec![],
            stakes: vec![],
        }
    }
    pub fn sign(&mut self, key: &Key) {
        self.public_key = key.public_key_bytes();
        self.signature = key.sign(&self.hash());
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        Key::verify(&self.public_key, &self.hash(), &self.signature)
    }
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&self.header()).unwrap())
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
    pub fn reward(&self, balance_staked: u128) -> u128 {
        self.fees() + util::reward(balance_staked)
    }
    pub fn transaction_hashes(&self) -> Vec<types::Hash> {
        let mut transaction_hashes = vec![];
        for transaction in self.transactions.iter() {
            transaction_hashes.push(transaction.hash());
        }
        transaction_hashes
    }
    pub fn stake_hashes(&self) -> Vec<types::Hash> {
        let mut stake_hashes = vec![];
        for stake in self.stakes.iter() {
            stake_hashes.push(stake.hash());
        }
        stake_hashes
    }
    pub fn merkle_root(hashes: &[types::Hash]) -> types::MerkleRoot {
        types::CBMT::build_merkle_root(hashes)
    }
    pub fn header(&self) -> Header {
        Header {
            previous_hash: self.previous_hash,
            transaction_merkle_root: Block::merkle_root(&self.transaction_hashes()),
            stake_merkle_root: Block::merkle_root(&self.stake_hashes()),
            timestamp: self.timestamp,
        }
    }
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.verify().is_err() {
            return Err("block signature".into());
        }
        let public_key_inputs = self.transactions.iter().map(|t| t.public_key_input).collect::<Vec<types::PublicKeyBytes>>();
        if (1..public_key_inputs.len()).any(|i| public_key_inputs[i..].contains(&public_key_inputs[i - 1])) {
            return Err("block includes multiple transactions from same public_key_input".into());
        }
        let public_keys = self.stakes.iter().map(|s| s.public_key).collect::<Vec<types::PublicKeyBytes>>();
        if (1..public_keys.len()).any(|i| public_keys[i..].contains(&public_keys[i - 1])) {
            return Err("block includes multiple stakes from same public_key".into());
        }
        Ok(())
    }
    pub fn validate_mint(&self) -> Result<(), Box<dyn Error>> {
        if self.verify().is_err() {
            return Err("block signature".into());
        }
        if !self.transactions.is_empty() {
            return Err("block mint transactions".into());
        }
        if self.stakes.len() != 1 {
            return Err("block mint stakes".into());
        }
        let stake = self.stakes.first().unwrap();
        stake.validate_mint()?;
        if stake.timestamp < self.timestamp {
            return Err("stake mint timestamp ancient".into());
        }
        Ok(())
    }
}
impl Default for Block {
    fn default() -> Self {
        Block {
            previous_hash: [0; 32],
            timestamp: 0,
            public_key: [0; 32],
            signature: [0; 64],
            transactions: vec![],
            stakes: vec![],
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        let block = Block {
            previous_hash: [0; 32],
            timestamp: 0,
            public_key: [0; 32],
            signature: [0; 64],
            transactions: vec![],
            stakes: vec![],
        };
        println!("{:x?}", block.hash());
        assert_eq!(
            [
                0xac, 0x6f, 0x86, 0xff, 0xf6, 0x30, 0xa5, 0x6a, 0x21, 0xf5, 0x9d, 0x3a, 0x0c, 0x1c, 0x69, 0x07, 0xfe, 0x3f, 0x7c, 0xaf, 0xd5, 0xfa, 0x91, 0x6f,
                0x9b, 0x72, 0x20, 0x32, 0xf6, 0x05, 0x9e, 0xd9
            ],
            block.hash()
        );
    }
}
