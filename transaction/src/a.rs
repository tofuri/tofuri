use crate::Error;
use crate::Transaction;
use crate::TransactionB;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_address::address;
use tofuri_key::Key;
use varint::Varint;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionA {
    pub input_address: [u8; 20],
    pub output_address: [u8; 20],
    pub amount: u128,
    pub fee: u128,
    pub timestamp: u32,
    pub hash: [u8; 32],
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}
impl TransactionA {
    pub fn b(&self) -> TransactionB {
        TransactionB {
            output_address: self.output_address,
            amount: Varint::from(self.amount),
            fee: Varint::from(self.fee),
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
    pub fn hash(&self) -> [u8; 32] {
        crate::hash(self)
    }
    pub fn sign(
        output_address: [u8; 20],
        amount: u128,
        fee: u128,
        timestamp: u32,
        key: &Key,
    ) -> Result<TransactionA, Error> {
        let mut transaction_a = TransactionA {
            input_address: [0; 20],
            output_address,
            amount: Varint::<4>::floor(amount),
            fee: Varint::<4>::floor(fee),
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
    fn get_output_address(&self) -> &[u8; 20] {
        &self.output_address
    }
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_amount_bytes(&self) -> [u8; 4] {
        Varint::from(self.amount).0
    }
    fn get_fee_bytes(&self) -> [u8; 4] {
        Varint::from(self.fee).0
    }
    fn hash(&self) -> [u8; 32] {
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
            .field("amount", &parseint::to_string::<18>(self.amount))
            .field("fee", &parseint::to_string::<18>(self.fee))
            .field("timestamp", &self.timestamp.to_string())
            .field("hash", &hex::encode(self.hash))
            .field("signature", &hex::encode(self.signature))
            .finish()
    }
}
