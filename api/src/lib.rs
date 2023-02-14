pub mod router;
use clap::Parser;
use pea_core::*;
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, value_parser, default_value = API)]
    pub api: String,
    /// API Internal Endpoint
    #[clap(long, value_parser, default_value = API_INTERNAL)]
    pub api_internal: String,
    /// Development mode
    #[clap(long, value_parser, default_value_t = false)]
    pub dev: bool,
}
