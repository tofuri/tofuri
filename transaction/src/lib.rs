use pea_core::{constants::AMOUNT_BYTES, types};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
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
    pub recovery_id: types::RecoveryId,
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
            recovery_id: types::RecoveryId,
            signature: String,
        }
        write!(
            f,
            "{:?}",
            Transaction {
                hash: hex::encode(self.hash()),
                input_address: pea_address::address::encode(&self.input().expect("valid input")),
                output_address: pea_address::address::encode(&self.output_address),
                amount: self.amount,
                fee: self.fee,
                timestamp: self.timestamp,
                recovery_id: self.recovery_id,
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
        if self.input()? == self.output_address {
            return Err("transaction input output".into());
        }
        Ok(())
    }
    pub fn metadata(&self) -> Metadata {
        Metadata {
            output_address: self.output_address,
            amount: pea_int::to_bytes(self.amount),
            fee: pea_int::to_bytes(self.fee),
            timestamp: self.timestamp,
            recovery_id: self.recovery_id,
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
            recovery_id: 0,
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
    pub recovery_id: types::RecoveryId,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Metadata {
    pub fn transaction(&self) -> Transaction {
        Transaction {
            output_address: self.output_address,
            amount: pea_int::from_bytes(&self.amount),
            fee: pea_int::from_bytes(&self.fee),
            timestamp: self.timestamp,
            recovery_id: self.recovery_id,
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
            Transaction::default().hash(),
            [
                42, 218, 131, 193, 129, 154, 83, 114, 218, 225, 35, 143, 193, 222, 209, 35, 200, 16, 79, 218, 161, 88, 98, 170, 238, 105, 66, 138, 24, 32, 252,
                218
            ]
        );
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(97, bincode::serialize(&Metadata::default()).unwrap().len());
    }
}
