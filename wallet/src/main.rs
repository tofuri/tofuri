use clap::Parser;
use pea_wallet::{command, Wallet};
use std::error::Error;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, value_parser, default_value = "http://localhost:9332")]
    pub http_api: String,
    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,
    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let wallet = Wallet::import(&args.wallet, &args.passphrase)?;
    command::clear();
    loop {
        command::main(&wallet, &args.http_api).await;
        command::press_any_key_to_continue();
    }
}
