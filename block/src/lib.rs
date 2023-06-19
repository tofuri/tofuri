use merkle_cbt::merkle_tree::Merge;
use merkle_cbt::CBMT as ExCBMT;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use sha2::Digest;
use sha2::Sha256;
use std::fmt;
use tofuri_key::Error;
use tofuri_key::Key;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
pub const GENESIS_BLOCK_BETA: [u8; 32] = [0; 32];
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub previous_hash: [u8; 32],
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
    pub transactions: Vec<Transaction>,
    pub stakes: Vec<Stake>,
}
impl Block {
    pub fn sign(
        previous_hash: [u8; 32],
        timestamp: u32,
        transactions: Vec<Transaction>,
        stakes: Vec<Stake>,
        key: &Key,
        previous_beta: &[u8; 32],
    ) -> Result<Block, Error> {
        let pi = key.vrf_prove(previous_beta)?;
        let mut block_b = Block {
            previous_hash,
            timestamp,
            pi,
            signature: [0; 64],
            transactions,
            stakes,
        };
        block_b.signature = key.sign(&block_b.hash())?;
        Ok(block_b)
    }
    pub fn input_address(&self) -> Result<[u8; 20], Error> {
        Ok(Key::address(&self.input_public_key()?))
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
    pub fn input_public_key(&self) -> Result<[u8; 33], Error> {
        Key::recover(&self.hash(), &self.signature)
    }
    pub fn hash(&self) -> [u8; 32] {
        let mut array = [0; 181];
        array[0..32].copy_from_slice(&self.previous_hash);
        array[32..64].copy_from_slice(&Block::merkle_root(&self.transaction_hashes()));
        array[64..96].copy_from_slice(&Block::merkle_root(&self.stake_hashes()));
        array[96..100].copy_from_slice(&self.timestamp.to_be_bytes());
        array[100..181].copy_from_slice(&self.pi);
        let mut hasher = Sha256::new();
        hasher.update(array);
        hasher.finalize().into()
    }
    pub fn merkle_root(hashes: &[[u8; 32]]) -> [u8; 32] {
        struct Hasher;
        impl Merge for Hasher {
            type Item = [u8; 32];
            fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
                let mut hasher = Sha256::new();
                hasher.update(left);
                hasher.update(right);
                hasher.finalize().into()
            }
        }
        <ExCBMT<[u8; 32], Hasher>>::build_merkle_root(hashes)
    }
    pub fn beta(&self) -> Result<[u8; 32], Error> {
        Key::vrf_proof_to_hash(&self.pi)
    }
}
impl Default for Block {
    fn default() -> Block {
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
impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Block")
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
            Block::default().hash(),
            [
                219, 36, 84, 162, 32, 189, 146, 241, 148, 53, 36, 177, 50, 142, 92, 103, 125, 225,
                26, 208, 20, 86, 5, 216, 113, 32, 54, 141, 75, 147, 221, 219
            ]
        );
    }
}
