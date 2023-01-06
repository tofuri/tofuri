use crate::constants::AMOUNT_BYTES;
use sha2::{Digest, Sha256};
pub type CompressedAmount = [u8; AMOUNT_BYTES];
pub type Hash = [u8; 32];
pub type Checksum = [u8; 4];
pub type MerkleRoot = [u8; 32];
pub type Beta = [u8; 32];
pub type Pi = [u8; 81];
pub type AddressBytes = [u8; 20];
pub type PublicKeyBytes = [u8; 33];
pub type SecretKeyBytes = [u8; 32];
pub type SignatureBytes = [u8; 64];
use merkle_cbt::{merkle_tree::Merge, CBMT as ExCBMT};
pub struct Hasher;
impl Merge for Hasher {
    type Item = [u8; 32];
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut hasher = Sha256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }
}
pub type CBMT = ExCBMT<[u8; 32], Hasher>;
