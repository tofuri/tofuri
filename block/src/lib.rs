use pea_core::{constants::COIN, types, util};
use pea_key::Key;
use pea_stake::Stake;
use pea_transaction::Transaction;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub previous_hash: types::Hash,
    pub transaction_merkle_root: types::MerkleRoot,
    pub stake_merkle_root: types::MerkleRoot,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Block {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
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
            address: String,
            signature: String,
            pi: String,
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
                address: pea_address::address::encode(&self.input_address().expect("valid input address")),
                signature: hex::encode(self.signature),
                pi: hex::encode(self.pi),
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
            signature: [0; 64],
            pi: [0; 81],
            transactions: vec![],
            stakes: vec![],
        }
    }
    pub fn sign(&mut self, key: &Key, previous_beta: &[u8]) {
        self.pi = key.vrf_prove(previous_beta).unwrap();
        self.signature = key.sign(&self.hash()).unwrap();
    }
    pub fn input_public_key(&self) -> Result<types::PublicKeyBytes, Box<dyn Error>> {
        Ok(Key::recover(&self.hash(), &self.signature)?)
    }
    pub fn input_address(&self) -> Result<types::AddressBytes, Box<dyn Error>> {
        Ok(util::address(&self.input_public_key()?))
    }
    pub fn beta(&self) -> Option<[u8; 32]> {
        Key::vrf_proof_to_hash(&self.pi)
    }
    pub fn verify(&self, previous_beta: &[u8]) -> Result<(), Box<dyn Error>> {
        let y = self.input_public_key()?;
        Key::vrf_verify(&y, &self.pi, previous_beta).ok_or("invalid proof")?;
        Ok(())
    }
    pub fn hash(&self) -> types::Hash {
        let mut hasher = Sha256::new();
        hasher.update(&bincode::serialize(&self.header()).unwrap());
        hasher.finalize().into()
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
    pub fn reward(&self) -> u128 {
        self.fees() + COIN
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
            pi: self.pi,
        }
    }
    pub fn validate(&self, previous_beta: &[u8]) -> Result<(), Box<dyn Error>> {
        if self.verify(previous_beta).is_err() {
            return Err("block signature".into());
        }
        let inputs = self
            .transactions
            .iter()
            .map(|t| t.input_address().expect("valid input address"))
            .collect::<Vec<types::AddressBytes>>();
        if (1..inputs.len()).any(|i| inputs[i..].contains(&inputs[i - 1])) {
            return Err("block includes multiple transactions from same input_public_key".into());
        }
        let inputs = self
            .stakes
            .iter()
            .map(|s| s.input_address().expect("valid input address"))
            .collect::<Vec<types::AddressBytes>>();
        if (1..inputs.len()).any(|i| inputs[i..].contains(&inputs[i - 1])) {
            return Err("block includes multiple stakes from same public_key".into());
        }
        Ok(())
    }
    pub fn validate_mint(&self, previous_beta: &[u8]) -> Result<(), Box<dyn Error>> {
        if self.verify(previous_beta).is_err() {
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
        if stake.timestamp != self.timestamp {
            return Err("stake mint timestamp".into());
        }
        Ok(())
    }
    pub fn metadata(&self) -> Metadata {
        Metadata {
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            signature: self.signature,
            pi: self.pi,
            transaction_hashes: self.transaction_hashes(),
            stake_hashes: self.stake_hashes(),
        }
    }
}
impl Default for Block {
    fn default() -> Self {
        Block {
            previous_hash: [0; 32],
            timestamp: 0,
            signature: [0; 64],
            pi: [0; 81],
            transactions: vec![],
            stakes: vec![],
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
    pub transaction_hashes: Vec<types::Hash>,
    pub stake_hashes: Vec<types::Hash>,
}
impl Metadata {
    pub fn block(&self, transactions: Vec<Transaction>, stakes: Vec<Stake>) -> Block {
        Block {
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            signature: self.signature,
            pi: self.pi,
            transactions,
            stakes,
        }
    }
}
impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            previous_hash: [0; 32],
            timestamp: 0,
            signature: [0; 64],
            pi: [0; 81],
            transaction_hashes: vec![],
            stake_hashes: vec![],
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        assert_eq!(
            Block::default().hash(),
            [219, 36, 84, 162, 32, 189, 146, 241, 148, 53, 36, 177, 50, 142, 92, 103, 125, 225, 26, 208, 20, 86, 5, 216, 113, 32, 54, 141, 75, 147, 221, 219]
        );
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(197, bincode::serialize(&Metadata::default()).unwrap().len());
    }
    #[test]
    fn test_u256_from_beta() {
        let key = Key::from_slice(&[0xcd; 32]);
        let mut block = Block::default();
        block.sign(&key, &[0; 32]);
        assert_eq!(
            util::u256(&block.beta().unwrap()),
            util::U256::from_dec_str("92526807160300854379423726328595779761032533927961162464096185194601493188181").unwrap()
        );
    }
}
