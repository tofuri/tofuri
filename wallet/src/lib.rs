pub mod cmd;
pub mod inquire;
use clap::Parser;
use reqwest::Url;
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, env = "API", default_value = "http://localhost:2021/")]
    pub api: Url,
}
