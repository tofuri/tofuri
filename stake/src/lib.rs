use pea_core::{types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub public_key: types::PublicKeyBytes,
    pub amount: u128,
    pub deposit: bool,
    pub fee: u128,
    pub timestamp: u32,
}
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
        util::hash(&bincode::serialize(&self.header()).unwrap())
    }
    pub fn sign(&mut self, key: &Key) {
        self.public_key = key.public_key_bytes();
        self.signature = key.sign(&self.hash());
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        Key::verify(&self.public_key, &self.hash(), &self.signature)
    }
    pub fn header(&self) -> Header {
        Header {
            public_key: self.public_key,
            amount: self.amount,
            deposit: self.deposit,
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
                228, 119, 38, 188, 114, 153, 96, 48, 45, 65, 119, 200, 241, 171, 244, 142, 232, 57, 219, 101, 144, 66, 253, 157, 184, 15, 199, 238, 250, 144,
                32, 138
            ]
        );
    }
}
