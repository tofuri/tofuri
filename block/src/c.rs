use crate::b::BlockB;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockC {
    pub previous_hash: [u8; 32],
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
    pub transaction_hashes: Vec<[u8; 32]>,
    pub stake_hashes: Vec<[u8; 32]>,
}
impl BlockC {
    pub fn b(&self, transactions: Vec<Transaction>, stakes: Vec<Stake>) -> BlockB {
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
    fn default() -> BlockC {
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
impl fmt::Debug for BlockC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockC")
            .field("previous_hash", &hex::encode(self.previous_hash))
            .field("timestamp", &self.timestamp)
            .field("signature", &hex::encode(self.signature))
            .field("pi", &hex::encode(self.pi))
            .field(
                "transaction_hashes",
                &self
                    .transaction_hashes
                    .iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>(),
            )
            .field(
                "stake_hashes",
                &self
                    .stake_hashes
                    .iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}
