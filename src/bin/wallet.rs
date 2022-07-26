use colored::*;
use axiom::{
    util,
    wallet::{command, Wallet},
};
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", "Wallet starting...".yellow());
    util::print::build();
    let wallet = Wallet::import()?;
    loop {
        command::main(&wallet).await?;
        command::press_any_key_to_continue();
    }
}
