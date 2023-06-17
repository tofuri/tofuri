use crate::Error;
use crate::Stake;
use crate::StakeB;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use std::fmt;
use tofuri_address::address;
use tofuri_core::*;
use tofuri_key::Key;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct StakeA {
    pub amount: u128,
    pub fee: u128,
    pub deposit: bool,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    pub input_address: [u8; 20],
    pub hash: [u8; 32],
}
impl StakeA {
    pub fn b(&self) -> StakeB {
        StakeB {
            amount: tofuri_int::to_be_bytes(self.amount),
            fee: tofuri_int::to_be_bytes(self.fee),
            deposit: self.deposit,
            timestamp: self.timestamp,
            signature: self.signature,
        }
    }
    pub fn hash(&self) -> [u8; 32] {
        crate::hash(self)
    }
    pub fn sign(
        deposit: bool,
        amount: u128,
        fee: u128,
        timestamp: u32,
        key: &Key,
    ) -> Result<StakeA, Error> {
        let mut stake_a = StakeA {
            amount: tofuri_int::floor(amount),
            fee: tofuri_int::floor(fee),
            deposit,
            timestamp,
            signature: [0; 64],
            input_address: [0; 20],
            hash: [0; 32],
        };
        stake_a.hash = stake_a.hash();
        stake_a.signature = key.sign(&stake_a.hash).map_err(Error::Key)?;
        stake_a.input_address = key.address_bytes();
        Ok(stake_a)
    }
}
impl Stake for StakeA {
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    fn get_deposit(&self) -> bool {
        self.deposit
    }
    fn get_fee_bytes(&self) -> [u8; AMOUNT_BYTES] {
        tofuri_int::to_be_bytes(self.fee)
    }
    fn hash(&self) -> [u8; 32] {
        crate::hash(self)
    }
    fn hash_input(&self) -> [u8; 9] {
        crate::hash_input(self)
    }
}
impl Default for StakeA {
    fn default() -> StakeA {
        StakeA {
            amount: 0,
            fee: 0,
            deposit: false,
            timestamp: 0,
            signature: [0; 64],
            input_address: [0; 20],
            hash: [0; 32],
        }
    }
}
impl fmt::Debug for StakeA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StakeA")
            .field("amount", &tofuri_int::to_string(self.amount))
            .field("fee", &tofuri_int::to_string(self.fee))
            .field("deposit", &self.deposit)
            .field("timestamp", &self.timestamp)
            .field("signature", &hex::encode(self.signature))
            .field("input_address", &address::encode(&self.input_address))
            .field("hash", &hex::encode(self.hash))
            .finish()
    }
}
