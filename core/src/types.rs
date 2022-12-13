use crate::constants::AMOUNT_BYTES;
pub type CompressedAmount = [u8; AMOUNT_BYTES];
pub type Hash = [u8; 32];
pub type Checksum = [u8; 4];
pub type MerkleRoot = [u8; 32];
pub type PublicKeyBytes = [u8; 32];
pub type SecretKeyBytes = [u8; 32];
pub type SignatureBytes = [u8; 64];
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
pub mod api {
    use serde::{Deserialize, Serialize};
    pub type Index = String;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Info {
        pub time: String,
        pub public_key: String,
        pub uptime: String,
        pub heartbeats: usize,
        pub tree_size: usize,
        pub lag: f64,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Sync {
        pub status: String,
        pub height: usize,
        pub last_seen: String,
    }
    pub type Height = usize;
    pub type Amount = u128;
    pub type Hash = String;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct State {
        pub balance: u128,
        pub balance_staked: u128,
        pub hashes: usize,
        pub latest_hashes: Vec<String>,
        pub stakers: Vec<String>,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Options {
        pub mint: bool,
        pub tempdb: bool,
        pub tempkey: bool,
        pub time_api: bool,
        pub trust: usize,
        pub pending: usize,
        pub ban_offline: usize,
        pub time_delta: u32,
        pub max_established: Option<u32>,
        pub tps: f64,
        pub bind_api: String,
        pub host: String,
        pub dev: bool,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Block {
        pub hash: String,
        pub previous_hash: String,
        pub timestamp: u32,
        pub public_key: String,
        pub signature: String,
        pub transactions: Vec<String>,
        pub stakes: Vec<String>,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Transaction {
        pub hash: String,
        pub public_key_input: String,
        pub public_key_output: String,
        pub amount: u128,
        pub fee: u128,
        pub timestamp: u32,
        pub signature: String,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Stake {
        pub hash: String,
        pub public_key: String,
        pub amount: u128,
        pub deposit: bool,
        pub fee: u128,
        pub timestamp: u32,
        pub signature: String,
    }
}
