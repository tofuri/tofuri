use pea_core::{constants::AMOUNT_BYTES, types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub output_address: types::AddressBytes,
    pub amount: types::CompressedAmount,
    pub fee: types::CompressedAmount,
    pub timestamp: u32,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub output_address: types::AddressBytes,
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
            input_address: String,
            output_address: String,
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
                input_address: pea_address::address::encode(&self.input_address().expect("valid input address")),
                output_address: pea_address::address::encode(&self.output_address),
                amount: self.amount,
                fee: self.fee,
                timestamp: self.timestamp,
                signature: hex::encode(self.signature),
            }
        )
    }
}
impl Transaction {
    pub fn new(public_key_output: types::AddressBytes, amount: u128, fee: u128, timestamp: u32) -> Transaction {
        Transaction {
            output_address: public_key_output,
            amount: pea_int::floor(amount),
            fee: pea_int::floor(fee),
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
    pub fn hash_input(&self) -> [u8; 32] {
        let mut bytes = [0; 32];
        bytes[0..20].copy_from_slice(&self.output_address);
        bytes[20..24].copy_from_slice(&self.timestamp.to_be_bytes());
        bytes[24..28].copy_from_slice(&pea_int::to_be_bytes(self.amount));
        bytes[28..32].copy_from_slice(&pea_int::to_be_bytes(self.fee));
        bytes
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
        if self.amount != pea_int::floor(self.amount) {
            return Err("transaction amount floor".into());
        }
        if self.fee != pea_int::floor(self.fee) {
            return Err("transaction fee floor".into());
        }
        if self.input_address()? == self.output_address {
            return Err("transaction input output".into());
        }
        Ok(())
    }
    pub fn metadata(&self) -> Metadata {
        Metadata {
            output_address: self.output_address,
            amount: pea_int::to_be_bytes(self.amount),
            fee: pea_int::to_be_bytes(self.fee),
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
}
impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            output_address: [0; 20],
            amount: 0,
            fee: 0,
            timestamp: 0,
            signature: [0; 64],
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub output_address: types::AddressBytes,
    pub amount: types::CompressedAmount,
    pub fee: types::CompressedAmount,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Metadata {
    pub fn transaction(&self) -> Transaction {
        Transaction {
            output_address: self.output_address,
            amount: pea_int::from_be_bytes(&self.amount),
            fee: pea_int::from_be_bytes(&self.fee),
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
}
impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            output_address: [0; 20],
            amount: [0; AMOUNT_BYTES],
            fee: [0; AMOUNT_BYTES],
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
            Transaction::default().hash(),
            [
                102, 104, 122, 173, 248, 98, 189, 119, 108, 143, 193, 139, 142, 159, 142, 32, 8, 151, 20, 133, 110, 226, 51, 179, 144, 42, 89, 29, 13, 95, 41,
                37
            ]
        );
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(96, bincode::serialize(&Metadata::default()).unwrap().len());
    }
}
