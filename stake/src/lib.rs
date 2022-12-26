use pea_core::{constants::AMOUNT_BYTES, types};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub fee: types::CompressedAmount,
    pub deposit: bool,
    pub timestamp: u32,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Stake {
    pub fee: u128,
    pub deposit: bool,
    pub timestamp: u32,
    pub recovery_id: types::RecoveryId,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl fmt::Debug for Stake {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #![allow(dead_code)]
        #[derive(Debug)]
        struct Stake {
            hash: String,
            address: String,
            fee: u128,
            deposit: bool,
            timestamp: u32,
            recovery_id: types::RecoveryId,
            signature: String,
        }
        write!(
            f,
            "{:?}",
            Stake {
                hash: hex::encode(self.hash()),
                address: pea_address::address::encode(&self.input().expect("valid input")),
                fee: self.fee,
                deposit: self.deposit,
                timestamp: self.timestamp,
                recovery_id: self.recovery_id,
                signature: hex::encode(self.signature),
            }
        )
    }
}
impl Stake {
    pub fn new(deposit: bool, fee: u128, timestamp: u32) -> Stake {
        Stake {
            fee: pea_int::floor(fee),
            deposit,
            timestamp,
            recovery_id: 0,
            signature: [0; 64],
        }
    }
    pub fn hash(&self) -> types::Hash {
        blake3::hash(&bincode::serialize(&self.header()).unwrap()).into()
    }
    pub fn sign(&mut self, key: &Key) {
        let (recovery_id, signature_bytes) = key.sign(&self.hash()).unwrap();
        self.recovery_id = recovery_id;
        self.signature = signature_bytes;
    }
    pub fn input(&self) -> Result<types::AddressBytes, Box<dyn Error>> {
        Ok(Key::recover(&self.hash(), &self.signature, self.recovery_id)?)
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        self.input()?;
        Ok(())
    }
    pub fn header(&self) -> Header {
        Header {
            fee: pea_int::to_bytes(self.fee),
            deposit: self.deposit,
            timestamp: self.timestamp,
        }
    }
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.verify().is_err() {
            return Err("stake signature".into());
        }
        if self.fee == 0 {
            return Err("stake fee zero".into());
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
        if self.fee != 0 {
            return Err("stake mint fee not zero".into());
        }
        if !self.deposit {
            return Err("stake mint deposit".into());
        }
        Ok(())
    }
    pub fn metadata(&self) -> Metadata {
        Metadata {
            fee: pea_int::to_bytes(self.fee),
            deposit: self.deposit,
            timestamp: self.timestamp,
            recovery_id: self.recovery_id,
            signature: self.signature,
        }
    }
}
impl Default for Stake {
    fn default() -> Self {
        Stake {
            fee: 0,
            deposit: false,
            timestamp: 0,
            recovery_id: 0,
            signature: [0; 64],
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub fee: types::CompressedAmount,
    pub deposit: bool,
    pub timestamp: u32,
    pub recovery_id: types::RecoveryId,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Metadata {
    pub fn stake(&self) -> Stake {
        Stake {
            fee: pea_int::from_bytes(&self.fee),
            deposit: self.deposit,
            timestamp: self.timestamp,
            recovery_id: self.recovery_id,
            signature: self.signature,
        }
    }
}
impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            fee: [0; AMOUNT_BYTES],
            deposit: false,
            timestamp: 0,
            recovery_id: 0,
            signature: [0; 64],
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        assert_eq!(
            Stake::default().hash(),
            [157, 21, 55, 43, 24, 48, 115, 90, 10, 45, 5, 33, 70, 105, 227, 39, 26, 117, 74, 172, 70, 254, 48, 59, 104, 187, 48, 70, 1, 58, 5, 116]
        );
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(74, bincode::serialize(&Metadata::default()).unwrap().len());
    }
}
