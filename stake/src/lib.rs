use pea_core::{constants::AMOUNT_BYTES, types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Clone)]
pub struct Stake {
    pub fee: u128,
    pub deposit: bool,
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
            address: String,
            fee: u128,
            deposit: bool,
            timestamp: u32,
            signature: String,
        }
        write!(
            f,
            "{:?}",
            Stake {
                hash: hex::encode(self.hash()),
                address: pea_address::address::encode(&self.input_address().expect("valid input address")),
                fee: self.fee,
                deposit: self.deposit,
                timestamp: self.timestamp,
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
            signature: [0; 64],
        }
    }
    pub fn hash(&self) -> types::Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.hash_input());
        hasher.finalize().into()
    }
    pub fn sign(&mut self, key: &Key) {
        self.signature = key.sign(&self.hash()).unwrap();
    }
    pub fn input_public_key(&self) -> Result<types::PublicKeyBytes, Box<dyn Error>> {
        Ok(Key::recover(&self.hash(), &self.signature)?)
    }
    pub fn input_address(&self) -> Result<types::AddressBytes, Box<dyn Error>> {
        Ok(util::address(&self.input_public_key()?))
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        self.input_public_key()?;
        Ok(())
    }
    pub fn hash_input(&self) -> [u8; 9] {
        let mut bytes = [0; 9];
        bytes[0..4].copy_from_slice(&self.timestamp.to_be_bytes());
        bytes[4..8].copy_from_slice(&pea_int::to_bytes(self.fee));
        bytes[8] = if self.deposit { 1 } else { 0 };
        bytes
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
            signature: [0; 64],
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub fee: types::CompressedAmount,
    pub deposit: bool,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Metadata {
    pub fn stake(&self) -> Stake {
        Stake {
            fee: pea_int::from_bytes(&self.fee),
            deposit: self.deposit,
            timestamp: self.timestamp,
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
            [
                62, 112, 119, 253, 47, 102, 214, 137, 224, 206, 230, 167, 207, 91, 55, 191, 45, 202, 124, 151, 154, 243, 86, 208, 163, 28, 188, 92, 133, 96,
                92, 125
            ]
        );
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(73, bincode::serialize(&Metadata::default()).unwrap().len());
    }
}
