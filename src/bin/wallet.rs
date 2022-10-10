use clap::Parser;
use colored::*;
use log::info;
use pea::{
    cli::WalletArgs,
    wallet::{command, Wallet},
};
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    print_build();
    let args = WalletArgs::parse();
    print_wallet_args(&args);
    let wallet = Wallet::import(&args.wallet, &args.passphrase)?;
    command::clear();
    loop {
        command::main(&wallet, &args.api).await;
        command::press_any_key_to_continue();
    }
}
pub fn print_build() {
    info!("{} {}", "Version".cyan(), env!("CARGO_PKG_VERSION"));
    info!("{} {}", "Commit".cyan(), env!("GIT_HASH"));
    info!("{} {}", "Repository".cyan(), env!("CARGO_PKG_REPOSITORY"));
}
pub fn print_wallet_args(args: &WalletArgs) {
    info!("{} {}", "--api".cyan(), args.api);
}
