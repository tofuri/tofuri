use crate::Error;
use crate::Transaction;
use crate::TransactionA;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_address::address;
use tofuri_core::*;
use tofuri_key::Key;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionB {
    pub output_address: [u8; 20],
    pub amount: [u8; AMOUNT_BYTES],
    pub fee: [u8; AMOUNT_BYTES],
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}
impl TransactionB {
    pub fn a(&self, input_address: Option<[u8; 20]>) -> Result<TransactionA, Error> {
        let input_address = input_address.unwrap_or(self.input_address()?);
        let transaction_a = TransactionA {
            output_address: self.output_address,
            amount: tofuri_int::from_be_slice(&self.amount),
            fee: tofuri_int::from_be_slice(&self.fee),
            timestamp: self.timestamp,
            signature: self.signature,
            input_address,
            hash: self.hash(),
        };
        Ok(transaction_a)
    }
    pub fn hash(&self) -> [u8; 32] {
        crate::hash(self)
    }
    pub fn input_address(&self) -> Result<[u8; 20], Error> {
        Ok(Key::address(&self.input_public_key()?))
    }
    pub fn input_public_key(&self) -> Result<[u8; 33], Error> {
        Key::recover(&self.hash(), &self.signature).map_err(Error::Key)
    }
}
impl Transaction for TransactionB {
    fn get_output_address(&self) -> &[u8; 20] {
        &self.output_address
    }
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_amount_bytes(&self) -> [u8; AMOUNT_BYTES] {
        self.amount
    }
    fn get_fee_bytes(&self) -> [u8; AMOUNT_BYTES] {
        self.fee
    }
    fn hash(&self) -> [u8; 32] {
        crate::hash(self)
    }
    fn hash_input(&self) -> [u8; 32] {
        crate::hash_input(self)
    }
}
impl Default for TransactionB {
    fn default() -> TransactionB {
        TransactionB {
            output_address: [0; 20],
            amount: [0; AMOUNT_BYTES],
            fee: [0; AMOUNT_BYTES],
            timestamp: 0,
            signature: [0; 64],
        }
    }
}
impl fmt::Debug for TransactionB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransactionB")
            .field("output_address", &address::encode(&self.output_address))
            .field("amount", &hex::encode(self.amount))
            .field("fee", &hex::encode(self.fee))
            .field("timestamp", &self.timestamp.to_string())
            .field("signature", &hex::encode(self.signature))
            .finish()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        assert_eq!(
            TransactionB::default().hash(),
            [
                102, 104, 122, 173, 248, 98, 189, 119, 108, 143, 193, 139, 142, 159, 142, 32, 8,
                151, 20, 133, 110, 226, 51, 179, 144, 42, 89, 29, 13, 95, 41, 37
            ]
        );
    }
}
