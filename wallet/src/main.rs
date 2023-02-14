use clap::Parser;
use pea_core::*;
use pea_wallet::wallet::clear;
use pea_wallet::wallet::press_any_key_to_continue;
use pea_wallet::wallet::Options;
use pea_wallet::wallet::Wallet;
#[tokio::main]
async fn main() {
    let mut args = pea_wallet::Args::parse();
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
