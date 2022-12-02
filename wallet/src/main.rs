use clap::Parser;
use colored::*;
use pea_wallet::command::{Command, Options};
use std::error::Error;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, value_parser, default_value = "http://localhost:9332")]
    pub api: String,
    /// Time synchronization requests to measure average delay
    #[clap(long, value_parser, default_value = "2")]
    pub time_sync_requests: usize,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "{} = {{ version = \"{}\" }}",
        env!("CARGO_PKG_NAME").yellow(),
        env!("CARGO_PKG_VERSION").magenta()
    );
    println!("{}/tree/{}", env!("CARGO_PKG_REPOSITORY").yellow(), env!("GIT_HASH").magenta());
    let args = Args::parse();
    let mut command = Command::new(Options {
        api: args.api,
        time_sync_requests: args.time_sync_requests,
    });
    command.sync_time().await;
    loop {
        if command.select().await {
            Command::press_any_key_to_continue();
        }
        Command::clear();
    }
}
