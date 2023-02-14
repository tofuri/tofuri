pub mod inquire;
pub mod wallet;
use clap::Parser;
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, value_parser, default_value = "http://localhost:80")]
    pub api: String,
}
