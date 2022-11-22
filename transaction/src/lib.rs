use pea_core::{types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: u128,
    pub fee: u128,
    pub timestamp: u32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: u128,
    pub fee: u128,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Transaction {
    pub fn new(public_key_output: types::PublicKeyBytes, amount: u128, fee: u128) -> Transaction {
        Transaction {
            public_key_input: [0; 32],
            public_key_output,
            amount,
            fee,
            timestamp: util::timestamp(),
            signature: [0; 64],
        }
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
            amount: self.amount,
            fee: self.fee,
            timestamp: self.timestamp,
        }
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
