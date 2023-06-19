use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use sha2::Digest;
use sha2::Sha256;
use std::fmt;
use tofuri_address::address;
use tofuri_key::Key;
use vint::vint;
use vint::Vint;
#[derive(Debug)]
pub enum Error {
    Key(tofuri_key::Error),
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub output_address: [u8; 20],
    pub amount: Vint<4>,
    pub fee: Vint<4>,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}
impl Transaction {
    pub fn sign(
        output_address: [u8; 20],
        amount: u128,
        fee: u128,
        timestamp: u32,
        key: &Key,
    ) -> Result<Transaction, Error> {
        let mut transaction = Transaction {
            output_address,
            amount: vint!(amount, 4),
            fee: vint!(fee, 4),
            timestamp,
            signature: [0; 64],
        };
        transaction.signature = key.sign(&transaction.hash()).map_err(Error::Key)?;
        Ok(transaction)
    }
    pub fn hash(&self) -> [u8; 32] {
        let mut array = [0; 32];
        array[0..20].copy_from_slice(&self.output_address);
        array[20..24].copy_from_slice(&self.timestamp.to_be_bytes());
        array[24..28].copy_from_slice(&self.amount.0);
        array[28..32].copy_from_slice(&self.fee.0);
        let mut hasher = Sha256::new();
        hasher.update(array);
        hasher.finalize().into()
    }
    pub fn input_address(&self) -> Result<[u8; 20], Error> {
        Ok(Key::address(&self.input_public_key()?))
    }
    pub fn input_public_key(&self) -> Result<[u8; 33], Error> {
        Key::recover(&self.hash(), &self.signature).map_err(Error::Key)
    }
}
impl Default for Transaction {
    fn default() -> Transaction {
        Transaction {
            output_address: [0; 20],
            amount: Vint([0; 4]),
            fee: Vint([0; 4]),
            timestamp: 0,
            signature: [0; 64],
        }
    }
}
impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Transaction")
            .field(
                "input_address",
                &address::encode(&self.input_address().unwrap()),
            )
            .field("output_address", &address::encode(&self.output_address))
            .field("amount", &parseint::to_string::<18>(self.amount.into()))
            .field("fee", &parseint::to_string::<18>(self.fee.into()))
            .field("timestamp", &self.timestamp.to_string())
            .field("hash", &hex::encode(self.hash()))
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
            Transaction::default().hash(),
            [
                102, 104, 122, 173, 248, 98, 189, 119, 108, 143, 193, 139, 142, 159, 142, 32, 8,
                151, 20, 133, 110, 226, 51, 179, 144, 42, 89, 29, 13, 95, 41, 37
            ]
        );
    }
}
