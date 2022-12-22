use pea_core::{constants::AMOUNT_BYTES, types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub input_public_key: types::PublicKeyBytes,
    pub output_address: types::AddressBytes,
    pub amount: types::CompressedAmount,
    pub fee: types::CompressedAmount,
    pub timestamp: u32,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub input_public_key: types::PublicKeyBytes,
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
            input_public_key: String,
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
                input_public_key: pea_address::public::encode(&self.input_public_key),
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
            input_public_key: [0; 32],
            output_address: public_key_output,
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
        self.input_public_key = key.public_key();
        self.signature = key.sign(&self.hash());
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        Key::verify(&self.input_public_key, &self.hash(), &self.signature)
    }
    pub fn header(&self) -> Header {
        Header {
            input_public_key: self.input_public_key,
            output_address: self.output_address,
            amount: pea_int::to_bytes(self.amount),
            fee: pea_int::to_bytes(self.fee),
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
        if self.amount != pea_int::floor(self.amount) {
            return Err("transaction amount floor".into());
        }
        if self.fee != pea_int::floor(self.fee) {
            return Err("transaction fee floor".into());
        }
        if util::address(&self.input_public_key) == self.output_address {
            return Err("transaction input output".into());
        }
        Ok(())
    }
    pub fn metadata(&self) -> Metadata {
        Metadata {
            input_public_key: self.input_public_key,
            output_address: self.output_address,
            amount: pea_int::to_bytes(self.amount),
            fee: pea_int::to_bytes(self.fee),
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
}
impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            input_public_key: [0; 32],
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
    pub input_public_key: types::PublicKeyBytes,
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
            input_public_key: self.input_public_key,
            output_address: self.output_address,
            amount: pea_int::from_bytes(&self.amount),
            fee: pea_int::from_bytes(&self.fee),
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
}
impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            input_public_key: [0; 32],
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
            [77, 0, 105, 118, 99, 106, 134, 150, 217, 9, 166, 48, 164, 8, 26, 173, 77, 124, 80, 248, 26, 253, 238, 4, 2, 11, 240, 80, 134, 171, 106, 85]
        );
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(128, bincode::serialize(&Metadata::default()).unwrap().len());
    }
}
