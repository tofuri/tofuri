use clap::Parser;
use pea::{
    cli::WalletArgs,
    print,
    wallet::{command, Wallet},
};
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    print::build();
    let args = WalletArgs::parse();
    print::wallet_args(&args);
    let wallet = Wallet::import(&args.wallet, &args.passphrase)?;
    command::clear();
    loop {
        command::main(&wallet, &args.api).await;
        command::press_any_key_to_continue();
    }
}
