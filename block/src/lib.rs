use pea_core::{constants::COIN, types, util};
use pea_key::Key;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Clone)]
pub struct BlockA {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
    pub transactions: Vec<TransactionB>,
    pub stakes: Vec<StakeB>,
    pub input_address: types::AddressBytes,
    pub beta: [u8; 32],
    pub hash: types::Hash,
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
}
#[derive(Serialize, Deserialize, Clone)]
pub struct BlockB {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
    pub transactions: Vec<TransactionB>,
    pub stakes: Vec<StakeB>,
}
impl fmt::Debug for BlockB {
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
impl BlockB {
    pub fn new(previous_hash: types::Hash, timestamp: u32) -> BlockB {
        BlockB {
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
        hasher.update(&self.hash_input());
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
    pub fn hash_input(&self) -> [u8; 181] {
        let mut bytes = [0; 181];
        bytes[0..32].copy_from_slice(&self.previous_hash);
        bytes[32..64].copy_from_slice(&BlockB::merkle_root(&self.transaction_hashes()));
        bytes[64..96].copy_from_slice(&BlockB::merkle_root(&self.stake_hashes()));
        bytes[96..100].copy_from_slice(&self.timestamp.to_be_bytes());
        bytes[100..181].copy_from_slice(&self.pi);
        bytes
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
            return Err("block includes multiple transactions from same input address".into());
        }
        let inputs = self
            .stakes
            .iter()
            .map(|s| s.input_address().expect("valid input address"))
            .collect::<Vec<types::AddressBytes>>();
        if (1..inputs.len()).any(|i| inputs[i..].contains(&inputs[i - 1])) {
            return Err("block includes multiple stakes from same input address".into());
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
    pub fn a(&self) -> BlockA {
        BlockA {
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            signature: self.signature,
            pi: self.pi,
            transactions: self.transactions.clone(),
            stakes: self.stakes.clone(),
            input_address: self.input_address().unwrap(),
            beta: self.beta().unwrap(),
            hash: self.hash(),
        }
    }
    pub fn c(&self) -> BlockC {
        BlockC {
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            signature: self.signature,
            pi: self.pi,
            transaction_hashes: self.transaction_hashes(),
            stake_hashes: self.stake_hashes(),
        }
    }
}
impl Default for BlockB {
    fn default() -> Self {
        BlockB {
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
pub struct BlockC {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
    pub transaction_hashes: Vec<types::Hash>,
    pub stake_hashes: Vec<types::Hash>,
}
impl BlockC {
    pub fn b(&self, transactions: Vec<TransactionB>, stakes: Vec<StakeB>) -> BlockB {
        BlockB {
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            signature: self.signature,
            pi: self.pi,
            transactions,
            stakes,
        }
    }
}
impl Default for BlockC {
    fn default() -> Self {
        BlockC {
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
            BlockB::default().hash(),
            [219, 36, 84, 162, 32, 189, 146, 241, 148, 53, 36, 177, 50, 142, 92, 103, 125, 225, 26, 208, 20, 86, 5, 216, 113, 32, 54, 141, 75, 147, 221, 219]
        );
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(197, bincode::serialize(&BlockC::default()).unwrap().len());
    }
    #[test]
    fn test_u256_from_beta() {
        let key = Key::from_slice(&[0xcd; 32]);
        let mut block = BlockB::default();
        block.sign(&key, &[0; 32]);
        assert_eq!(
            util::u256(&block.beta().unwrap()),
            util::U256::from_dec_str("92526807160300854379423726328595779761032533927961162464096185194601493188181").unwrap()
        );
    }
}
