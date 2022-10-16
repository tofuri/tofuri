use pea_wallet::{command, Wallet};
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = WalletArgs::parse();
    let wallet = Wallet::import(&args.wallet, &args.passphrase)?;
    command::clear();
    loop {
        command::main(&wallet, &args.api).await;
        command::press_any_key_to_continue();
    }
}
use clap::Parser;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct WalletArgs {
    /// Multiaddr to a validator in the network
    #[clap(long, value_parser, default_value = "http://localhost:8080")]
    pub api: String,
    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,
    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,
}
