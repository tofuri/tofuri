pub mod router;
use clap::Parser;
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
