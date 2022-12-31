use pea_core::{constants::COIN, types, util};
use pea_key::Key;
use pea_stake::{StakeA, StakeB};
use pea_transaction::{TransactionA, TransactionB};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};
use std::error::Error;
pub trait Block {
    fn get_previous_hash(&self) -> &types::Hash;
    fn get_merkle_root_transaction(&self) -> types::MerkleRoot;
    fn get_merkle_root_stake(&self) -> types::MerkleRoot;
    fn get_timestamp(&self) -> u32;
    fn get_pi(&self) -> &types::Pi;
    fn hash(&self) -> types::Hash;
    fn hash_input(&self) -> [u8; 181];
    fn beta(&self) -> Result<types::Beta, Box<dyn Error>>;
}
impl Block for BlockA {
    fn get_previous_hash(&self) -> &types::Hash {
        &self.previous_hash
    }
    fn get_merkle_root_transaction(&self) -> types::MerkleRoot {
        merkle_root(&self.transaction_hashes())
    }
    fn get_merkle_root_stake(&self) -> types::MerkleRoot {
        merkle_root(&self.stake_hashes())
    }
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_pi(&self) -> &types::Pi {
        &self.pi
    }
    fn hash(&self) -> types::Hash {
        hash(self)
    }
    fn hash_input(&self) -> [u8; 181] {
        hash_input(self)
    }
    fn beta(&self) -> Result<types::Beta, Box<dyn Error>> {
        beta(self)
    }
}
impl Block for BlockB {
    fn get_previous_hash(&self) -> &types::Hash {
        &self.previous_hash
    }
    fn get_merkle_root_transaction(&self) -> types::MerkleRoot {
        merkle_root(&self.transaction_hashes())
    }
    fn get_merkle_root_stake(&self) -> types::MerkleRoot {
        merkle_root(&self.stake_hashes())
    }
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_pi(&self) -> &types::Pi {
        &self.pi
    }
    fn hash(&self) -> types::Hash {
        hash(self)
    }
    fn hash_input(&self) -> [u8; 181] {
        hash_input(self)
    }
    fn beta(&self) -> Result<types::Beta, Box<dyn Error>> {
        beta(self)
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockA {
    pub hash: types::Hash,
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    pub beta: [u8; 32],
    #[serde(with = "BigArray")]
    pub pi: types::Pi,
    #[serde(with = "BigArray")]
    pub input_public_key: types::PublicKeyBytes,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    pub transactions: Vec<TransactionA>,
    pub stakes: Vec<StakeA>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockB {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    #[serde(with = "BigArray")]
    pub pi: types::Pi,
    pub transactions: Vec<TransactionB>,
    pub stakes: Vec<StakeB>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockC {
    pub previous_hash: types::Hash,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    #[serde(with = "BigArray")]
    pub pi: types::Pi,
    pub transaction_hashes: Vec<types::Hash>,
    pub stake_hashes: Vec<types::Hash>,
}
impl BlockA {
    pub fn b(&self) -> BlockB {
        let mut transactions = vec![];
        let mut stakes = vec![];
        for transaction in self.transactions.iter() {
            transactions.push(transaction.b())
        }
        for stake in self.stakes.iter() {
            stakes.push(stake.b());
        }
        BlockB {
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            signature: self.signature,
            pi: self.pi,
            transactions,
            stakes,
        }
    }
    pub fn hash(&self) -> types::Hash {
        hash(self)
    }
    pub fn sign(
        previous_hash: types::Hash,
        timestamp: u32,
        transactions: Vec<TransactionA>,
        stakes: Vec<StakeA>,
        key: &Key,
        previous_beta: &[u8],
    ) -> Result<BlockA, Box<dyn Error>> {
        let pi = key.vrf_prove(previous_beta).ok_or("failed to generate proof")?;
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
        block_a.signature = key.sign(&block_a.hash)?;
        block_a.input_public_key = key.public_key_bytes();
        Ok(block_a)
    }
    pub fn input_address(&self) -> types::AddressBytes {
        util::address(&self.input_public_key)
    }
    pub fn reward(&self) -> u128 {
        self.fees() + COIN
    }
    fn fees(&self) -> u128 {
        let mut fees = 0;
        for transaction in self.transactions.iter() {
            fees += transaction.fee;
        }
        for stake in self.stakes.iter() {
            fees += stake.fee;
        }
        fees
    }
    fn transaction_hashes(&self) -> Vec<types::Hash> {
        self.transactions.iter().map(|x| x.hash()).collect()
    }
    fn stake_hashes(&self) -> Vec<types::Hash> {
        self.stakes.iter().map(|x| x.hash()).collect()
    }
}
impl BlockB {
    pub fn a(&self) -> Result<BlockA, Box<dyn Error>> {
        let mut transactions = vec![];
        let mut stakes = vec![];
        for transaction in self.transactions.iter() {
            transactions.push(transaction.a(None)?)
        }
        for stake in self.stakes.iter() {
            stakes.push(stake.a(None)?);
        }
        Ok(BlockA {
            hash: self.hash(),
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            beta: self.beta()?,
            pi: self.pi,
            input_public_key: self.input_public_key()?,
            signature: self.signature,
            transactions,
            stakes,
        })
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
    pub fn hash(&self) -> types::Hash {
        hash(self)
    }
    fn transaction_hashes(&self) -> Vec<types::Hash> {
        self.transactions.iter().map(|x| x.hash()).collect()
    }
    fn stake_hashes(&self) -> Vec<types::Hash> {
        self.stakes.iter().map(|x| x.hash()).collect()
    }
    fn input_public_key(&self) -> Result<types::PublicKeyBytes, Box<dyn Error>> {
        Ok(Key::recover(&self.hash(), &self.signature)?)
    }
}
impl BlockC {
    pub fn a(
        &self,
        transactions: Vec<TransactionA>,
        stakes: Vec<StakeA>,
        beta: Option<[u8; 32]>,
        input_public_key: Option<types::PublicKeyBytes>,
    ) -> Result<BlockA, Box<dyn Error>> {
        let block_b = self.b(vec![], vec![]);
        let beta = match beta {
            Some(x) => x,
            None => block_b.beta()?,
        };
        let input_public_key = match input_public_key {
            Some(x) => x,
            None => block_b.input_public_key()?,
        };
        let mut block_a = BlockA {
            hash: [0; 32],
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            beta,
            pi: self.pi,
            input_public_key,
            signature: self.signature,
            transactions,
            stakes,
        };
        block_a.hash = block_a.hash();
        Ok(block_a)
    }
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
impl Default for BlockA {
    fn default() -> Self {
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
fn hash<T: Block>(block: &T) -> types::Hash {
    let mut hasher = Sha256::new();
    hasher.update(&block.hash_input());
    hasher.finalize().into()
}
fn hash_input<T: Block>(block: &T) -> [u8; 181] {
    let mut bytes = [0; 181];
    bytes[0..32].copy_from_slice(block.get_previous_hash());
    bytes[32..64].copy_from_slice(&block.get_merkle_root_transaction());
    bytes[64..96].copy_from_slice(&block.get_merkle_root_stake());
    bytes[96..100].copy_from_slice(&block.get_timestamp().to_be_bytes());
    bytes[100..181].copy_from_slice(block.get_pi());
    bytes
}
fn merkle_root(hashes: &[types::Hash]) -> types::MerkleRoot {
    types::CBMT::build_merkle_root(hashes)
}
fn beta<T: Block>(block: &T) -> Result<types::Beta, Box<dyn Error>> {
    Key::vrf_proof_to_hash(block.get_pi()).ok_or("invalid beta".into())
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
        let block_a = BlockA::sign([0; 32], 0, vec![], vec![], &key, &[0; 32]).unwrap();
        assert_eq!(
            util::u256(&block_a.beta),
            util::U256::from_dec_str("92526807160300854379423726328595779761032533927961162464096185194601493188181").unwrap()
        );
    }
}
