pub mod router;
use clap::Parser;
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, value_parser, default_value = "localhost:9332")]
    pub api: String,
}
