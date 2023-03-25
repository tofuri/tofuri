use clap::Parser;
use tofuri_core::*;
use tofuri_wallet::clear;
use tofuri_wallet::press_any_key_to_continue;
use tofuri_wallet::Wallet;
use tofuri_wallet::CARGO_PKG_NAME;
use tofuri_wallet::CARGO_PKG_REPOSITORY;
use tofuri_wallet::CARGO_PKG_VERSION;
#[tokio::main]
async fn main() {
    let mut args = tofuri_wallet::Args::parse();
    println!(
        "{}",
        tofuri_util::build(CARGO_PKG_NAME, CARGO_PKG_VERSION, CARGO_PKG_REPOSITORY)
    );
    if args.dev && args.api == HTTP_API {
        args.api = HTTP_API_DEV.to_string();
    }
    let mut wallet = Wallet::new(args);
    loop {
        if wallet.select().await {
            press_any_key_to_continue();
        }
        clear();
    }
}
