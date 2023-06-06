use crate::Error;
use crate::Transaction;
use crate::TransactionB;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_address::address;
use tofuri_core::*;
use tofuri_key::Key;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionA {
    pub input_address: AddressBytes,
    pub output_address: AddressBytes,
    pub amount: u128,
    pub fee: u128,
    pub timestamp: u32,
    pub hash: Hash,
    #[serde(with = "BigArray")]
    pub signature: SignatureBytes,
}
impl TransactionA {
    pub fn b(&self) -> TransactionB {
        TransactionB {
            output_address: self.output_address,
            amount: tofuri_int::to_be_bytes(self.amount),
            fee: tofuri_int::to_be_bytes(self.fee),
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
    pub fn hash(&self) -> Hash {
        crate::hash(self)
    }
    pub fn sign(
        output_address: AddressBytes,
        amount: u128,
        fee: u128,
        timestamp: u32,
        key: &Key,
    ) -> Result<TransactionA, Error> {
        let mut transaction_a = TransactionA {
            input_address: [0; 20],
            output_address,
            amount: tofuri_int::floor(amount),
            fee: tofuri_int::floor(fee),
            timestamp,
            hash: [0; 32],
            signature: [0; 64],
        };
        transaction_a.hash = transaction_a.hash();
        transaction_a.signature = key.sign(&transaction_a.hash).map_err(Error::Key)?;
        transaction_a.input_address = key.address_bytes();
        Ok(transaction_a)
    }
}
impl Transaction for TransactionA {
    fn get_output_address(&self) -> &AddressBytes {
        &self.output_address
    }
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_amount_bytes(&self) -> AmountBytes {
        tofuri_int::to_be_bytes(self.amount)
    }
    fn get_fee_bytes(&self) -> AmountBytes {
        tofuri_int::to_be_bytes(self.fee)
    }
    fn hash(&self) -> Hash {
        crate::hash(self)
    }
    fn hash_input(&self) -> [u8; 32] {
        crate::hash_input(self)
    }
}
impl Default for TransactionA {
    fn default() -> TransactionA {
        TransactionA {
            output_address: [0; 20],
            amount: 0,
            fee: 0,
            timestamp: 0,
            signature: [0; 64],
            input_address: [0; 20],
            hash: [0; 32],
        }
    }
}
impl fmt::Debug for TransactionA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransactionA")
            .field("input_address", &address::encode(&self.input_address))
            .field("output_address", &address::encode(&self.output_address))
            .field("amount", &tofuri_int::to_string(self.amount))
            .field("fee", &tofuri_int::to_string(self.fee))
            .field("timestamp", &self.timestamp.to_string())
            .field("hash", &hex::encode(self.hash))
            .field("signature", &hex::encode(self.signature))
            .finish()
    }
}
