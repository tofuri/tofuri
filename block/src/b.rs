use crate::a::BlockA;
use crate::c::BlockC;
use crate::Block;
use crate::Error;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_core::*;
use tofuri_key::Key;
use tofuri_stake::StakeB;
use tofuri_transaction::TransactionB;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockB {
    pub previous_hash: Hash,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: SignatureBytes,
    #[serde(with = "BigArray")]
    pub pi: Pi,
    pub transactions: Vec<TransactionB>,
    pub stakes: Vec<StakeB>,
}
impl BlockB {
    pub fn a(&self) -> Result<BlockA, Error> {
        let mut transactions = vec![];
        let mut stakes = vec![];
        for transaction in self.transactions.iter() {
            transactions.push(transaction.a(None).map_err(Error::Transaction)?)
        }
        for stake in self.stakes.iter() {
            stakes.push(stake.a(None).map_err(Error::Stake)?);
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
            stakes,
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
    pub fn transaction_hashes(&self) -> Vec<Hash> {
        self.transactions.iter().map(|x| x.hash()).collect()
    }
    pub fn stake_hashes(&self) -> Vec<Hash> {
        self.stakes.iter().map(|x| x.hash()).collect()
    }
    pub fn input_public_key(&self) -> Result<PublicKeyBytes, Error> {
        Key::recover(&self.hash(), &self.signature).map_err(Error::Key)
    }
}
impl Block for BlockB {
    fn get_previous_hash(&self) -> &Hash {
        &self.previous_hash
    }
    fn get_merkle_root_transaction(&self) -> MerkleRoot {
        crate::merkle_root(&self.transaction_hashes())
    }
    fn get_merkle_root_stake(&self) -> MerkleRoot {
        crate::merkle_root(&self.stake_hashes())
    }
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_pi(&self) -> &Pi {
        &self.pi
    }
    fn hash(&self) -> Hash {
        crate::hash(self)
    }
    fn hash_input(&self) -> [u8; 181] {
        crate::hash_input(self)
    }
    fn beta(&self) -> Result<Beta, Error> {
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
