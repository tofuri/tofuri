use pea_core::{constants::AMOUNT_BYTES, types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};
use std::error::Error;
pub trait Stake {
    fn get_timestamp(&self) -> u32;
    fn get_deposit(&self) -> bool;
    fn get_fee(&self) -> u128;
    fn hash(&self) -> types::Hash;
    fn hash_input(&self) -> [u8; 9];
}
impl Stake for StakeA {
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_deposit(&self) -> bool {
        self.deposit
    }
    fn get_fee(&self) -> u128 {
        self.fee
    }
    fn hash(&self) -> types::Hash {
        hash(self)
    }
    fn hash_input(&self) -> [u8; 9] {
        hash_input(self)
    }
}
impl Stake for StakeB {
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_deposit(&self) -> bool {
        self.deposit
    }
    fn get_fee(&self) -> u128 {
        self.fee
    }
    fn hash(&self) -> types::Hash {
        hash(self)
    }
    fn hash_input(&self) -> [u8; 9] {
        hash_input(self)
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StakeA {
    pub fee: u128,
    pub deposit: bool,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    pub input_address: types::AddressBytes,
    pub hash: types::Hash,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StakeB {
    pub fee: u128,
    pub deposit: bool,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StakeC {
    pub fee: types::CompressedAmount,
    pub deposit: bool,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl StakeA {
    pub fn hash(&self) -> types::Hash {
        hash(self)
    }
    pub fn b(&self) -> StakeB {
        StakeB {
            fee: self.fee,
            deposit: self.deposit,
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
}
impl StakeB {
    pub fn hash(&self) -> types::Hash {
        hash(self)
    }
    pub fn a(&self, input_address: Option<types::AddressBytes>) -> Result<StakeA, Box<dyn Error>> {
        let input_address = match input_address {
            Some(x) => x,
            None => self.input_address()?,
        };
        Ok(StakeA {
            fee: self.fee,
            deposit: self.deposit,
            timestamp: self.timestamp,
            signature: self.signature,
            input_address,
            hash: self.hash(),
        })
    }
    pub fn c(&self) -> StakeC {
        StakeC {
            fee: pea_int::to_be_bytes(self.fee),
            deposit: self.deposit,
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
    pub fn sign(deposit: bool, fee: u128, timestamp: u32, key: &Key) -> Result<StakeB, Box<dyn Error>> {
        let mut stake_b = StakeB {
            fee: pea_int::floor(fee),
            deposit,
            timestamp,
            signature: [0; 64],
        };
        stake_b.signature = key.sign(&stake_b.hash())?;
        Ok(stake_b)
    }
    fn input_address(&self) -> Result<types::AddressBytes, Box<dyn Error>> {
        Ok(util::address(&self.input_public_key()?))
    }
    fn input_public_key(&self) -> Result<types::PublicKeyBytes, Box<dyn Error>> {
        Ok(Key::recover(&self.hash(), &self.signature)?)
    }
}
fn hash<T: Stake>(stake: &T) -> types::Hash {
    let mut hasher = Sha256::new();
    hasher.update(&stake.hash_input());
    hasher.finalize().into()
}
fn hash_input<T: Stake>(stake: &T) -> [u8; 9] {
    let mut bytes = [0; 9];
    bytes[0..4].copy_from_slice(&stake.get_timestamp().to_be_bytes());
    bytes[4..8].copy_from_slice(&pea_int::to_be_bytes(stake.get_fee()));
    bytes[8] = if stake.get_deposit() { 1 } else { 0 };
    bytes
}
impl StakeC {
    pub fn b(&self) -> StakeB {
        StakeB {
            fee: pea_int::from_be_bytes(&self.fee),
            deposit: self.deposit,
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
}
impl Default for StakeA {
    fn default() -> Self {
        StakeA {
            fee: 0,
            deposit: false,
            timestamp: 0,
            signature: [0; 64],
            input_address: [0; 20],
            hash: [0; 32],
        }
    }
}
impl Default for StakeB {
    fn default() -> Self {
        StakeB {
            fee: 0,
            deposit: false,
            timestamp: 0,
            signature: [0; 64],
        }
    }
}
impl Default for StakeC {
    fn default() -> Self {
        StakeC {
            fee: [0; AMOUNT_BYTES],
            deposit: false,
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
            StakeB::default().hash(),
            [
                62, 112, 119, 253, 47, 102, 214, 137, 224, 206, 230, 167, 207, 91, 55, 191, 45, 202, 124, 151, 154, 243, 86, 208, 163, 28, 188, 92, 133, 96,
                92, 125
            ]
        );
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(73, bincode::serialize(&StakeC::default()).unwrap().len());
    }
}
