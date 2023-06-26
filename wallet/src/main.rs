use clap::Parser;
use reqwest::Client;
use wallet::clear;
use wallet::press_any_key_to_continue;
use wallet::Args;
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = Client::new();
    let mut key = None;
    loop {
        if wallet::select(&client, args.api.as_str(), &mut key).await {
            press_any_key_to_continue();
        }
        clear();
    }
}
