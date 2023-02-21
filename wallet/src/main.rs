use clap::Parser;
use pea_core::*;
use pea_wallet::wallet::clear;
use pea_wallet::wallet::press_any_key_to_continue;
use pea_wallet::wallet::Options;
use pea_wallet::wallet::Wallet;
use pea_wallet::CARGO_PKG_NAME;
use pea_wallet::CARGO_PKG_REPOSITORY;
use pea_wallet::CARGO_PKG_VERSION;
#[tokio::main]
async fn main() {
    let mut args = pea_wallet::Args::parse();
    println!("{}", pea_util::build(CARGO_PKG_NAME, CARGO_PKG_VERSION, CARGO_PKG_REPOSITORY));
    if args.dev {
        if args.api == HTTP_API {
            args.api = DEV_HTTP_API.to_string();
        }
    }
    let mut wallet = Wallet::new(Options { api: args.api });
    loop {
        if wallet.select().await {
            press_any_key_to_continue();
        }
        clear();
    }
}
