pub mod router;
pub mod util;
use clap::Parser;
use serde::Deserialize;
use serde::Serialize;
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, env = "API", default_value = "[::]:2022")]
    pub api: String,

    /// API Internal Endpoint
    #[clap(long, env = "RPC", default_value = "[::]:2021")]
    pub rpc: String,

    /// Disable tracing_subscriber timestamps
    #[clap(long, env = "WITHOUT_TIME")]
    pub without_time: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub cargo_pkg_name: String,
    pub cargo_pkg_version: String,
    pub cargo_pkg_repository: String,
    pub git_hash: String,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub input_address: String,
    pub output_address: String,
    pub amount: String,
    pub fee: String,
    pub timestamp: u32,
    pub hash: String,
    pub signature: String,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stake {
    pub amount: String,
    pub fee: String,
    pub deposit: bool,
    pub timestamp: u32,
    pub signature: String,
    pub input_address: String,
    pub hash: String,
}
