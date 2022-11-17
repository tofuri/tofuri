use pea_core::{types, util};
use pea_key::Key;
use pea_stake::Stake;
use pea_transaction::Transaction;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::{error::Error, fmt};
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
                public_key: hex::encode(self.public_key),
                signature: hex::encode(self.signature),
                transactions: self.transactions.iter().map(|x| hex::encode(x.hash())).collect(),
                stakes: self.stakes.iter().map(|x| hex::encode(x.hash())).collect(),
            }
        )
    }
}
impl Block {
    pub fn from(
        previous_hash: types::Hash,
        timestamp: u32,
        public_key: types::PublicKeyBytes,
        signature: types::SignatureBytes,
        transactions: Vec<Transaction>,
        stakes: Vec<Stake>,
    ) -> Block {
        Block {
            previous_hash,
            timestamp,
            public_key,
            signature,
            transactions,
            stakes,
        }
    }
    pub fn new(previous_hash: types::Hash) -> Block {
        Block::from(previous_hash, util::timestamp(), [0; 32], [0; 64], vec![], vec![])
    }
    pub fn new_timestamp_0(previous_hash: types::Hash) -> Block {
        Block::from(previous_hash, 0, [0; 32], [0; 64], vec![], vec![])
    }
    pub fn sign(&mut self, key: &Key) {
        self.public_key = key.public_key_bytes();
        self.signature = key.sign(&self.hash());
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        Key::verify(&self.public_key, &self.hash(), &self.signature)
    }
    pub fn hash(&self) -> types::Hash {
        let block_metadata = Metadata::from(self);
        util::hash(&bincode::serialize(&Header::from(&block_metadata)).unwrap())
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
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub previous_hash: types::Hash,
    pub transaction_merkle_root: types::MerkleRoot,
    pub stake_merkle_root: types::MerkleRoot,
    pub timestamp: u32,
}
impl Header {
    pub fn from(block: &Metadata) -> Header {
        Header {
            previous_hash: block.previous_hash,
            transaction_merkle_root: block.transaction_merkle_root,
            stake_merkle_root: block.stake_merkle_root,
            timestamp: block.timestamp,
        }
    }
}
#[derive(Debug)]
pub struct Metadata {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    pub public_key: types::PublicKeyBytes,
    pub signature: types::SignatureBytes,
    pub transaction_hashes: Vec<types::Hash>,
    pub transaction_merkle_root: types::MerkleRoot,
    pub stake_hashes: Vec<types::Hash>,
    pub stake_merkle_root: types::MerkleRoot,
}
impl Metadata {
    pub fn from(block: &Block) -> Metadata {
        let transaction_hashes = Metadata::transaction_hashes(&block.transactions);
        let stake_hashes = Metadata::stake_hashes(&block.stakes);
        Metadata {
            previous_hash: block.previous_hash,
            timestamp: block.timestamp,
            public_key: block.public_key,
            signature: block.signature,
            transaction_merkle_root: Metadata::merkle_root(&transaction_hashes),
            transaction_hashes,
            stake_merkle_root: Metadata::merkle_root(&stake_hashes),
            stake_hashes,
        }
    }
    fn transaction_hashes(transactions: &Vec<Transaction>) -> Vec<types::Hash> {
        let mut transaction_hashes = vec![];
        for transaction in transactions {
            transaction_hashes.push(transaction.hash());
        }
        transaction_hashes
    }
    fn stake_hashes(stakes: &Vec<Stake>) -> Vec<types::Hash> {
        let mut stake_hashes = vec![];
        for stake in stakes {
            stake_hashes.push(stake.hash());
        }
        stake_hashes
    }
    fn merkle_root(hashes: &[types::Hash]) -> types::MerkleRoot {
        types::CBMT::build_merkle_root(hashes)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataLean {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    pub public_key: types::PublicKeyBytes,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    pub transaction_hashes: Vec<types::Hash>,
    pub stake_hashes: Vec<types::Hash>,
}
impl MetadataLean {
    pub fn from(block_metadata: &Metadata) -> MetadataLean {
        MetadataLean {
            previous_hash: block_metadata.previous_hash,
            timestamp: block_metadata.timestamp,
            public_key: block_metadata.public_key,
            signature: block_metadata.signature,
            transaction_hashes: block_metadata.transaction_hashes.to_vec(),
            stake_hashes: block_metadata.stake_hashes.to_vec(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        let block = Block::from([0; 32], 0, [0; 32], [0; 64], vec![], vec![]);
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
