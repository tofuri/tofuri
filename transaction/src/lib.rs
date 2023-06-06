mod a;
mod b;
pub use a::TransactionA;
pub use b::TransactionB;
use sha2::Digest;
use sha2::Sha256;
use tofuri_core::*;
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
