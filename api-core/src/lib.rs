use serde::Deserialize;
use serde::Serialize;
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Root {
    pub cargo_pkg_name: String,
    pub cargo_pkg_version: String,
    pub cargo_pkg_repository: String,
    pub git_hash: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Block {
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: u32,
    pub beta: String,
    pub pi: String,
    pub forger_address: String,
    pub signature: String,
    pub transactions: Vec<String>,
    pub stakes: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Transaction {
    pub input_address: String,
    pub output_address: String,
    pub amount: String,
    pub fee: String,
    pub timestamp: u32,
    pub hash: String,
    pub signature: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Stake {
    pub amount: String,
    pub fee: String,
    pub deposit: bool,
    pub timestamp: u32,
    pub signature: String,
    pub input_address: String,
    pub hash: String,
}
