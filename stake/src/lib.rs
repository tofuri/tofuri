mod a;
mod b;
pub use a::StakeA;
pub use b::StakeB;
use sha2::Digest;
use sha2::Sha256;
use tofuri_core::*;
#[derive(Debug)]
pub enum Error {
    Key(tofuri_key::Error),
}
pub trait Stake {
    fn get_timestamp(&self) -> u32;
    fn get_deposit(&self) -> bool;
    fn get_fee_bytes(&self) -> AmountBytes;
    fn hash(&self) -> Hash;
    fn hash_input(&self) -> [u8; 9];
}
pub fn hash<T: Stake>(stake: &T) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(stake.hash_input());
    hasher.finalize().into()
}
pub fn hash_input<T: Stake>(stake: &T) -> [u8; 9] {
    let mut bytes = [0; 9];
    bytes[0..4].copy_from_slice(&stake.get_timestamp().to_be_bytes());
    bytes[4..8].copy_from_slice(&stake.get_fee_bytes());
    bytes[8] = if stake.get_deposit() { 1 } else { 0 };
    bytes
}
