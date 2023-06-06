mod a;
mod b;
mod c;
pub use a::BlockA;
pub use b::BlockB;
pub use c::BlockC;
use merkle_cbt::merkle_tree::Merge;
use merkle_cbt::CBMT as ExCBMT;
use sha2::Digest;
use sha2::Sha256;
use tofuri_core::*;
use tofuri_key::Key;
#[derive(Debug)]
pub enum Error {
    Key(tofuri_key::Error),
    Transaction(tofuri_transaction::Error),
    Stake(tofuri_stake::Error),
}
pub trait Block {
    fn get_previous_hash(&self) -> &Hash;
    fn get_merkle_root_transaction(&self) -> MerkleRoot;
    fn get_merkle_root_stake(&self) -> MerkleRoot;
    fn get_timestamp(&self) -> u32;
    fn get_pi(&self) -> &Pi;
    fn hash(&self) -> Hash;
    fn hash_input(&self) -> [u8; 181];
    fn beta(&self) -> Result<Beta, Error>;
}
fn hash<T: Block>(block: &T) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(block.hash_input());
    hasher.finalize().into()
}
fn hash_input<T: Block>(block: &T) -> [u8; 181] {
    let mut bytes = [0; 181];
    bytes[0..32].copy_from_slice(block.get_previous_hash());
    bytes[32..64].copy_from_slice(&block.get_merkle_root_transaction());
    bytes[64..96].copy_from_slice(&block.get_merkle_root_stake());
    bytes[96..100].copy_from_slice(&block.get_timestamp().to_be_bytes());
    bytes[100..181].copy_from_slice(block.get_pi());
    bytes
}
fn merkle_root(hashes: &[Hash]) -> MerkleRoot {
    struct Hasher;
    impl Merge for Hasher {
        type Item = [u8; 32];
        fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
            let mut hasher = Sha256::new();
            hasher.update(left);
            hasher.update(right);
            hasher.finalize().into()
        }
    }
    <ExCBMT<[u8; 32], Hasher>>::build_merkle_root(hashes)
}
fn beta<T: Block>(block: &T) -> Result<Beta, Error> {
    Key::vrf_proof_to_hash(block.get_pi()).map_err(Error::Key)
}
