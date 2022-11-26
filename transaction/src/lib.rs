use pea_core::{types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: types::CompressedAmount,
    pub fee: types::CompressedAmount,
    pub timestamp: u32,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: u128,
    pub fee: u128,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #![allow(dead_code)]
        #[derive(Debug)]
        struct Transaction {
            hash: String,
            public_key_input: String,
            public_key_output: String,
            amount: u128,
            fee: u128,
            timestamp: u32,
            signature: String,
        }
        write!(
            f,
            "{:?}",
            Transaction {
                hash: hex::encode(self.hash()),
                public_key_input: pea_address::public::encode(&self.public_key_input),
                public_key_output: pea_address::public::encode(&self.public_key_output),
                amount: self.amount,
                fee: self.fee,
                timestamp: self.timestamp,
                signature: hex::encode(self.signature),
            }
        )
    }
}
impl Transaction {
    pub fn new(public_key_output: types::PublicKeyBytes, amount: u128, fee: u128) -> Result<Transaction, Box<dyn Error>> {
        if amount != pea_amount::floor(&amount) {
            return Err("Invalid amount".into());
        }
        if fee != pea_amount::floor(&fee) {
            return Err("Invalid fee".into());
        }
        Ok(Transaction {
            public_key_input: [0; 32],
            public_key_output,
            amount,
            fee,
            timestamp: util::timestamp(),
            signature: [0; 64],
        })
    }
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&self.header()).unwrap())
    }
    pub fn sign(&mut self, key: &Key) {
        self.public_key_input = key.public_key_bytes();
        self.signature = key.sign(&self.hash());
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        Key::verify(&self.public_key_input, &self.hash(), &self.signature)
    }
    pub fn header(&self) -> Header {
        Header {
            public_key_input: self.public_key_input,
            public_key_output: self.public_key_output,
            amount: pea_amount::to_bytes(&self.amount),
            fee: pea_amount::to_bytes(&self.fee),
            timestamp: self.timestamp,
        }
    }
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.verify().is_err() {
            return Err("transaction signature".into());
        }
        if self.amount == 0 {
            return Err("transaction amount zero".into());
        }
        if self.fee == 0 {
            return Err("transaction fee zero".into());
        }
        if self.amount != pea_amount::floor(&self.amount) {
            return Err("transaction amount floor".into());
        }
        if self.fee != pea_amount::floor(&self.fee) {
            return Err("transaction fee floor".into());
        }
        if self.timestamp > util::timestamp() {
            return Err("transaction timestamp future".into());
        }
        if self.public_key_input == self.public_key_output {
            return Err("transaction input output".into());
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        let transaction = Transaction {
            public_key_input: [0; 32],
            public_key_output: [0; 32],
            amount: 0,
            fee: 0,
            timestamp: 0,
            signature: [0; 64],
        };
        assert_eq!(
            transaction.hash(),
            [
                172, 111, 134, 255, 246, 48, 165, 106, 33, 245, 157, 58, 12, 28, 105, 7, 254, 63, 124, 175, 213, 250, 145, 111, 155, 114, 32, 50, 246, 5, 158,
                217
            ]
        );
    }
}
