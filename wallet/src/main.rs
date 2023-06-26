use clap::Parser;
use colored::*;
use reqwest::Client;
use wallet::cmd;
use wallet::Args;
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = Client::new();
    let mut key = None;
    loop {
        match cmd::select(&client, args.api.as_str(), &mut key).await {
            Ok(true) => cmd::press_any_key_to_continue(),
            Ok(false) => {}
            Err(e) => println!("{}", e.to_string().red()),
        }
        cmd::clear();
    }
}
