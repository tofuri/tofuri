use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use sha2::Digest;
use sha2::Sha256;
use tofuri_core::*;
use tofuri_key::Key;
#[derive(Debug)]
pub enum Error {
    Key(tofuri_key::Error),
}
pub trait Transaction {
    fn get_output_address(&self) -> &AddressBytes;
    fn get_timestamp(&self) -> u32;
    fn get_amount_bytes(&self) -> AmountBytes;
    fn get_fee_bytes(&self) -> AmountBytes;
    fn hash(&self) -> Hash;
    fn hash_input(&self) -> [u8; 32];
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
        hash(self)
    }
    fn hash_input(&self) -> [u8; 32] {
        hash_input(self)
    }
}
impl Transaction for TransactionB {
    fn get_output_address(&self) -> &AddressBytes {
        &self.output_address
    }
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_amount_bytes(&self) -> AmountBytes {
        self.amount
    }
    fn get_fee_bytes(&self) -> AmountBytes {
        self.fee
    }
    fn hash(&self) -> Hash {
        hash(self)
    }
    fn hash_input(&self) -> [u8; 32] {
        hash_input(self)
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionB {
    pub output_address: AddressBytes,
    pub amount: AmountBytes,
    pub fee: AmountBytes,
    pub timestamp: u32,
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
        hash(self)
    }
    pub fn sign(
        public_key_output: AddressBytes,
        amount: u128,
        fee: u128,
        timestamp: u32,
        key: &Key,
    ) -> Result<TransactionA, Error> {
        let mut transaction_a = TransactionA {
            input_address: [0; 20],
            output_address: public_key_output,
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
impl TransactionB {
    pub fn a(&self, input_address: Option<AddressBytes>) -> Result<TransactionA, Error> {
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
    pub fn hash(&self) -> Hash {
        hash(self)
    }
    fn input_address(&self) -> Result<AddressBytes, Error> {
        Ok(Key::address(&self.input_public_key()?))
    }
    fn input_public_key(&self) -> Result<PublicKeyBytes, Error> {
        Key::recover(&self.hash(), &self.signature).map_err(Error::Key)
    }
}
fn hash<T: Transaction>(transaction: &T) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(transaction.hash_input());
    hasher.finalize().into()
}
fn hash_input<T: Transaction>(transaction: &T) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[0..20].copy_from_slice(transaction.get_output_address());
    bytes[20..24].copy_from_slice(&transaction.get_timestamp().to_be_bytes());
    bytes[24..28].copy_from_slice(&transaction.get_amount_bytes());
    bytes[28..32].copy_from_slice(&transaction.get_fee_bytes());
    bytes
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
