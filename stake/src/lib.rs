use pea_core::{types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stake {
    pub public_key: types::PublicKeyBytes,
    pub amount: u128,
    pub deposit: bool, // false -> withdraw
    pub fee: u128,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Stake {
    pub fn new(deposit: bool, amount: u128, fee: u128) -> Stake {
        Stake {
            public_key: [0; 32],
            amount,
            deposit,
            fee,
            timestamp: util::timestamp(),
            signature: [0; 64],
        }
    }
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&Header::from(self)).unwrap())
    }
    pub fn sign(&mut self, key: &Key) {
        self.public_key = key.public_key_bytes();
        self.signature = key.sign(&self.hash());
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        Key::verify(&self.public_key, &self.hash(), &self.signature)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub public_key: types::PublicKeyBytes,
    pub amount: u128,
    pub fee: u128,
    pub timestamp: u32,
}
impl Header {
    pub fn from(stake: &Stake) -> Header {
        Header {
            public_key: stake.public_key,
            amount: stake.amount,
            fee: stake.fee,
            timestamp: stake.timestamp,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Compressed {
    pub public_key: types::PublicKeyBytes,
    pub amount: types::CompressedAmount,
    pub fee: types::CompressedAmount,
    pub deposit: bool,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        let stake = Stake {
            public_key: [0; 32],
            amount: 0,
            deposit: false,
            fee: 0,
            timestamp: 0,
            signature: [0; 64],
        };
        assert_eq!(
            stake.hash(),
            [
                21, 94, 12, 116, 214, 170, 54, 153, 102, 153, 156, 138, 151, 46, 61, 146, 230, 38, 102, 86, 253, 116, 8, 127, 164, 101, 49, 219, 69, 41, 101,
                245
            ]
        );
    }
}
