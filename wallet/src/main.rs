use clap::Parser;
use colored::*;
use pea_wallet::command::Command;
use std::error::Error;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, value_parser, default_value = "http://localhost:9332")]
    pub api: String,
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
    let mut command = Command::new(args.api);
    loop {
        if command.select().await {
            Command::press_any_key_to_continue();
        }
        Command::clear();
    }
}
