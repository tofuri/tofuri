use clap::Parser;
use tofuri_wallet::clear;
use tofuri_wallet::press_any_key_to_continue;
use tofuri_wallet::Args;
use tofuri_wallet::Wallet;
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut wallet = Wallet::new(args);
    loop {
        if wallet.select().await {
            press_any_key_to_continue();
        }
        clear();
    }
}
