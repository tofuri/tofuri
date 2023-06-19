use crate::a::BlockA;
use crate::b::BlockB;
use crate::Block;
use crate::Error;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_stake::Stake;
use tofuri_transaction::TransactionA;
use tofuri_transaction::TransactionB;
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
    pub fn a(
        &self,
        transactions: Vec<TransactionA>,
        stakes: Vec<Stake>,
        beta: Option<[u8; 32]>,
        input_public_key: Option<[u8; 33]>,
    ) -> Result<BlockA, Error> {
        let block_b = self.b(transactions.iter().map(|x| x.b()).collect(), stakes.clone());
        let beta = beta.unwrap_or(block_b.beta()?);
        let input_public_key = input_public_key.unwrap_or(block_b.input_public_key()?);
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
    pub fn b(&self, transactions: Vec<TransactionB>, stakes: Vec<Stake>) -> BlockB {
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
