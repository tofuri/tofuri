use pea_core::{constants::MIN_STAKE, types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub public_key: types::PublicKeyBytes,
    pub amount: types::CompressedAmount,
    pub deposit: bool,
    pub fee: types::CompressedAmount,
    pub timestamp: u32,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Stake {
    pub public_key: types::PublicKeyBytes,
    pub deposit: bool,
    pub amount: u128,
    pub fee: u128,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl fmt::Debug for Stake {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #![allow(dead_code)]
        #[derive(Debug)]
        struct Stake {
            hash: String,
            public_key: String,
            deposit: bool,
            amount: u128,
            fee: u128,
            timestamp: u32,
            signature: String,
        }
        write!(
            f,
            "{:?}",
            Stake {
                hash: hex::encode(self.hash()),
                public_key: pea_address::public::encode(&self.public_key),
                deposit: self.deposit,
                amount: self.amount,
                fee: self.fee,
                timestamp: self.timestamp,
                signature: hex::encode(self.signature),
            }
        )
    }
}
impl Stake {
    pub fn new(deposit: bool, amount: u128, fee: u128, timestamp: u32) -> Stake {
        Stake {
            public_key: [0; 32],
            deposit,
            amount: pea_int::floor(amount),
            fee: pea_int::floor(fee),
            timestamp,
            signature: [0; 64],
        }
    }
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&self.header()).unwrap())
    }
    pub fn sign(&mut self, key: &Key) {
        self.public_key = key.public_key();
        self.signature = key.sign(&self.hash());
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        Key::verify(&self.public_key, &self.hash(), &self.signature)
    }
    pub fn header(&self) -> Header {
        Header {
            public_key: self.public_key,
            deposit: self.deposit,
            amount: pea_int::to_bytes(self.amount),
            fee: pea_int::to_bytes(self.fee),
            timestamp: self.timestamp,
        }
    }
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.verify().is_err() {
            return Err("stake signature".into());
        }
        if self.amount == 0 {
            return Err("stake amount zero".into());
        }
        if self.fee == 0 {
            return Err("stake fee zero".into());
        }
        if self.amount != pea_int::floor(self.amount) {
            return Err("stake amount floor".into());
        }
        if self.fee != pea_int::floor(self.fee) {
            return Err("stake fee floor".into());
        }
        Ok(())
    }
    pub fn validate_mint(&self) -> Result<(), Box<dyn Error>> {
        if self.verify().is_err() {
            return Err("stake mint signature".into());
        }
        if self.amount != MIN_STAKE {
            return Err("stake mint amount not MIN_STAKE".into());
        }
        if self.fee != 0 {
            return Err("stake mint fee not zero".into());
        }
        if !self.deposit {
            return Err("stake mint deposit".into());
        }
        Ok(())
    }
}
impl Default for Stake {
    fn default() -> Self {
        Stake {
            public_key: [0; 32],
            deposit: false,
            amount: 0,
            fee: 0,
            timestamp: 0,
            signature: [0; 64],
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        let stake = Stake::default();
        assert_eq!(
            stake.hash(),
            [38, 150, 39, 44, 192, 60, 233, 141, 22, 106, 60, 125, 33, 31, 169, 224, 58, 198, 123, 28, 152, 25, 113, 88, 196, 152, 101, 43, 21, 185, 28, 134]
        );
    }
}
