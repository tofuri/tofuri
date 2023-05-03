pub mod router;
use clap::Parser;
pub const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CARGO_PKG_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, env = "API", default_value = "[::]:2022")]
    pub api: String,

    /// API Internal Endpoint
    #[clap(long, env = "RPC", default_value = "[::]:2021")]
    pub rpc: String,
}
