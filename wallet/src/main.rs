use clap::Parser;
use wallet::clear;
use wallet::press_any_key_to_continue;
use wallet::Args;
use wallet::Wallet;
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut wallet = Wallet::new(args.api);
    loop {
        if wallet.select().await {
            press_any_key_to_continue();
        }
        clear();
    }
}
