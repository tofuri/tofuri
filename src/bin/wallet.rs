use axiom::{
    util::{self, print, WalletArgs},
    wallet::{command, Wallet},
};
use clap::Parser;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    util::print::build();
    let args = WalletArgs::parse();
    print::wallet_args(&args);
    let wallet = Wallet::import();
    print::clear();
    loop {
        command::main(&wallet, &args.api).await?;
        command::press_any_key_to_continue();
    }
}
