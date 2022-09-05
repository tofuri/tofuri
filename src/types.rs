pub use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature};
use std::collections::VecDeque;
pub type Height = usize;
pub type Heartbeats = usize;
pub type Timestamp = u32;
pub type Amount = u64;
pub type Hash = [u8; 32];
pub type Checksum = [u8; 4];
pub type MerkleRoot = [u8; 32];
pub type PublicKeyBytes = [u8; 32];
pub type SecretKeyBytes = [u8; 32];
pub type SignatureBytes = [u8; 64];
pub type Hashes = Vec<Hash>;
pub type Staker = (PublicKeyBytes, Height);
pub type Stakers = VecDeque<Staker>;
use merkle_cbt::{merkle_tree::Merge, CBMT as ExCBMT};
pub struct Hasher;
impl Merge for Hasher {
    type Item = [u8; 32];
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut hasher = blake3::Hasher::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }
}
pub type CBMT = ExCBMT<[u8; 32], Hasher>;
