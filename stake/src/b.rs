use crate::Error;
use crate::Stake;
use crate::StakeA;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_int::AMOUNT_BYTES;
use tofuri_key::Key;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct StakeB {
    pub amount: [u8; AMOUNT_BYTES],
    pub fee: [u8; AMOUNT_BYTES],
    pub deposit: bool,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}
impl StakeB {
    pub fn a(&self, input_address: Option<[u8; 20]>) -> Result<StakeA, Error> {
        let input_address = input_address.unwrap_or(self.input_address()?);
        let stake_a = StakeA {
            amount: tofuri_int::from_be_slice(&self.amount),
            fee: tofuri_int::from_be_slice(&self.fee),
            deposit: self.deposit,
            timestamp: self.timestamp,
            signature: self.signature,
            input_address,
            hash: self.hash(),
        };
        Ok(stake_a)
    }
    pub fn hash(&self) -> [u8; 32] {
        crate::hash(self)
    }
    fn input_address(&self) -> Result<[u8; 20], Error> {
        let input_address = Key::address(&self.input_public_key()?);
        Ok(input_address)
    }
    fn input_public_key(&self) -> Result<[u8; 33], Error> {
        Key::recover(&self.hash(), &self.signature).map_err(Error::Key)
    }
}
impl Stake for StakeB {
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_deposit(&self) -> bool {
        self.deposit
    }
    fn get_fee_bytes(&self) -> [u8; AMOUNT_BYTES] {
        self.fee
    }
    fn hash(&self) -> [u8; 32] {
        crate::hash(self)
    }
    fn hash_input(&self) -> [u8; 9] {
        crate::hash_input(self)
    }
}
impl Default for StakeB {
    fn default() -> StakeB {
        StakeB {
            amount: [0; AMOUNT_BYTES],
            fee: [0; AMOUNT_BYTES],
            deposit: false,
            timestamp: 0,
            signature: [0; 64],
        }
    }
}
impl fmt::Debug for StakeB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StakeB")
            .field("amount", &hex::encode(self.amount))
            .field("fee", &hex::encode(self.fee))
            .field("deposit", &self.deposit)
            .field("timestamp", &self.timestamp)
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
            StakeB::default().hash(),
            [
                62, 112, 119, 253, 47, 102, 214, 137, 224, 206, 230, 167, 207, 91, 55, 191, 45,
                202, 124, 151, 154, 243, 86, 208, 163, 28, 188, 92, 133, 96, 92, 125
            ]
        );
    }
}
