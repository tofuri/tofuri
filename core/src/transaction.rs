use crate::types;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: types::Amount,
    pub fee: types::Amount,
    pub timestamp: types::Timestamp,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: types::Amount,
    pub fee: types::Amount,
    pub timestamp: types::Timestamp,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Compressed {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: types::CompressedAmount,
    pub fee: types::CompressedAmount,
    pub timestamp: types::Timestamp,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
