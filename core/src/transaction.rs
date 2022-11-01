use crate::{types, util};
use ed25519::signature::Signer;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
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
impl Transaction {
    pub fn new(public_key_output: types::PublicKeyBytes, amount: types::Amount, fee: types::Amount) -> Transaction {
        Transaction {
            public_key_input: [0; 32],
            public_key_output,
            amount,
            fee,
            timestamp: util::timestamp(),
            signature: [0; 64],
        }
    }
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&Header::from(self)).unwrap())
    }
    pub fn sign(&mut self, keypair: &types::Keypair) {
        self.public_key_input = keypair.public.to_bytes();
        self.signature = keypair.sign(&self.hash()).to_bytes();
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        let public_key = types::PublicKey::from_bytes(&self.public_key_input)?;
        let signature = types::Signature::from_bytes(&self.signature)?;
        Ok(public_key.verify_strict(&self.hash(), &signature)?)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: types::Amount,
    pub fee: types::Amount,
    pub timestamp: types::Timestamp,
}
impl Header {
    pub fn from(transaction: &Transaction) -> Header {
        Header {
            public_key_input: transaction.public_key_input,
            public_key_output: transaction.public_key_output,
            amount: transaction.amount,
            fee: transaction.fee,
            timestamp: transaction.timestamp,
        }
    }
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
