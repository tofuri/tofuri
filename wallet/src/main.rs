use clap::Parser;
use colored::*;
use pea_wallet::wallet::clear;
use pea_wallet::wallet::press_any_key_to_continue;
use pea_wallet::wallet::Options;
use pea_wallet::wallet::Wallet;
use std::error::Error;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, value_parser, default_value = "localhost:9332")]
    pub api: String,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "{} = {{ version = \"{}\" }}",
        env!("CARGO_PKG_NAME").yellow(),
        env!("CARGO_PKG_VERSION").magenta()
    );
    println!("{}/tree/{}", env!("CARGO_PKG_REPOSITORY").yellow(), env!("GIT_HASH").magenta());
    let args = Args::parse();
    let mut wallet = Wallet::new(Options { api: args.api });
    loop {
        if wallet.select().await {
            press_any_key_to_continue();
        }
        clear();
    }
}
