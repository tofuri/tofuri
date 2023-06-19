use crate::a::BlockA;
use crate::c::BlockC;
use crate::Block;
use crate::Error;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_key::Key;
use tofuri_stake::Stake;
use tofuri_transaction::TransactionB;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockB {
    pub previous_hash: [u8; 32],
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
    pub transactions: Vec<TransactionB>,
    pub stakes: Vec<Stake>,
}
impl BlockB {
    pub fn a(&self) -> Result<BlockA, Error> {
        let mut transactions = vec![];
        for transaction in self.transactions.iter() {
            transactions.push(transaction.a(None).map_err(Error::Transaction)?)
        }
        let block_a = BlockA {
            hash: self.hash(),
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            beta: self.beta()?,
            pi: self.pi,
            input_public_key: self.input_public_key()?,
            signature: self.signature,
            transactions,
            stakes: self.stakes.clone(),
        };
        Ok(block_a)
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
    pub fn transaction_hashes(&self) -> Vec<[u8; 32]> {
        self.transactions.iter().map(|x| x.hash()).collect()
    }
    pub fn stake_hashes(&self) -> Vec<[u8; 32]> {
        self.stakes.iter().map(|x| x.hash()).collect()
    }
    pub fn input_public_key(&self) -> Result<[u8; 33], Error> {
        Key::recover(&self.hash(), &self.signature).map_err(Error::Key)
    }
}
impl Block for BlockB {
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
impl Default for BlockB {
    fn default() -> BlockB {
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
impl fmt::Debug for BlockB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockB")
            .field("previous_hash", &hex::encode(self.previous_hash))
            .field("timestamp", &self.timestamp)
            .field("signature", &hex::encode(self.signature))
            .field("pi", &hex::encode(self.pi))
            .field("transactions", &self.transactions)
            .field("stakes", &self.stakes)
            .finish()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        assert_eq!(
            BlockB::default().hash(),
            [
                219, 36, 84, 162, 32, 189, 146, 241, 148, 53, 36, 177, 50, 142, 92, 103, 125, 225,
                26, 208, 20, 86, 5, 216, 113, 32, 54, 141, 75, 147, 221, 219
            ]
        );
    }
}
