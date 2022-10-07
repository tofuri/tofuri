use crate::{
    address, amount,
    api::{get, post},
    constants::DECIMAL_PRECISION,
    print,
    stake::Stake,
    transaction::Transaction,
    types,
    wallet::Wallet,
};
use chrono::{Local, TimeZone};
use colored::*;
use inquire::{Confirm, CustomType, Select};
use std::{
    error::Error,
    io::{stdin, stdout, Write},
    process,
};
use termion::{input::TermRead, raw::IntoRawMode};
pub async fn main(wallet: &Wallet, api: &str) -> Result<(), Box<dyn Error>> {
    match Select::new(
        ">>",
        vec![
            "Address",
            "Search",
            "Key",
            "Data",
            "Balance",
            "Height",
            "Transaction",
            "Stake",
            "Validator",
            "Exit",
        ],
    )
    .prompt()
    .unwrap_or_else(|err| {
        println!("{}", err.to_string().red());
        process::exit(0)
    }) {
        "Address" => address(wallet),
        "Search" => search(api).await,
        "Key" => key(wallet),
        "Data" => data(wallet),
        "Balance" => balance(api, &wallet.address()).await,
        "Height" => height(api).await,
        "Transaction" => transaction(api, wallet).await,
        "Stake" => stake(api, wallet).await,
        "Validator" => validator(api).await,
        "Exit" => exit(),
        _ => {}
    }
    Ok(())
}
pub fn press_any_key_to_continue() {
    println!("Press any key to continue...");
    let mut stdout = stdout().into_raw_mode().unwrap();
    stdout.flush().unwrap();
    stdin().events().next();
    print::clear();
}
async fn validator(api: &str) {
    match get::index(api).await {
        Ok(info) => println!("{}", info.green()),
        Err(err) => println!("{}", err.to_string().red()),
    };
}
async fn balance(api: &str, address: &str) {
    match get::balance(api, address).await {
        Ok(balance) => match get::balance_staked(api, address).await {
            Ok(balance_staked) => println!(
                "Account balance: {}, locked: {}.",
                (balance as f64 / DECIMAL_PRECISION as f64)
                    .to_string()
                    .yellow(),
                (balance_staked as f64 / DECIMAL_PRECISION as f64)
                    .to_string()
                    .yellow()
            ),
            Err(err) => println!("{}", err.to_string().red()),
        },
        Err(err) => println!("{}", err.to_string().red()),
    };
}
async fn height(api: &str) {
    match get::height(api).await {
        Ok(height) => println!("Latest block height is {}.", height.to_string().yellow()),
        Err(err) => println!("{}", err.to_string().red()),
    };
}
async fn transaction(api: &str, wallet: &Wallet) {
    let address = CustomType::<String>::new("Address:")
        .with_error_message("Please enter a valid address")
        .with_help_message("Type the hex encoded address with 0x as prefix")
        .with_parser(&|x| match address::decode(x) {
            Ok(y) => Ok(address::encode(&y)),
            Err(_) => Err(()),
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        });
    let amount = (CustomType::<f64>::new("Amount:")
        .with_formatter(&|i| format!("{:.18} pea", i)) // DECIMAL_PRECISION
        .with_error_message("Please type a valid number")
        .with_help_message("Type the amount in pea using a decimal point as a separator")
        .with_parser(&|x| match x.parse::<f64>() {
            Ok(f) => Ok(
                amount::round(&((f * DECIMAL_PRECISION as f64) as u128)) as f64
                    / DECIMAL_PRECISION as f64,
            ),
            Err(_) => Err(()),
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
        * DECIMAL_PRECISION as f64) as types::Amount;
    let fee = CustomType::<types::Amount>::new("Fee:")
        .with_formatter(&|i| format!("{} {}", i, if i == 1 { "satoshi" } else { "satoshis" }))
        .with_error_message("Please type a valid number")
        .with_help_message("Type the amount in satoshis using a decimal point as a separator")
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        });
    if !match Confirm::new("Send?").prompt() {
        Ok(b) => b,
        Err(err) => {
            println!("{}", err.to_string().red());
            process::exit(0)
        }
    } {
        return;
    }
    let mut transaction = Transaction::new(address::decode(&address).unwrap(), amount, fee);
    transaction.sign(&wallet.keypair);
    println!("Hash: {}", hex::encode(transaction.hash()).cyan());
    match post::transaction(api, &transaction).await {
        Ok(res) => println!(
            "{}",
            if res == "success" {
                res.green()
            } else {
                res.red()
            }
        ),
        Err(err) => println!("{}", err.to_string().red()),
    };
}
async fn stake(api: &str, wallet: &Wallet) {
    let deposit = match Select::new(">>", vec!["deposit", "withdraw"])
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        }) {
        "deposit" => true,
        "withdraw" => false,
        _ => false,
    };
    let amount = (CustomType::<f64>::new("Amount:")
        .with_formatter(&|i| format!("{:.18} pea", i)) // DECIMAL_PRECISION
        .with_error_message("Please type a valid number")
        .with_help_message("Type the amount in pea using a decimal point as a separator")
        .with_parser(&|x| match x.parse::<f64>() {
            Ok(f) => Ok(
                amount::round(&((f * DECIMAL_PRECISION as f64) as u128)) as f64
                    / DECIMAL_PRECISION as f64,
            ),
            Err(_) => Err(()),
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
        * DECIMAL_PRECISION as f64) as types::Amount;
    let fee = CustomType::<types::Amount>::new("Fee:")
        .with_formatter(&|i| format!("{} {}", i, if i == 1 { "satoshi" } else { "satoshis" }))
        .with_error_message("Please type a valid number")
        .with_help_message("Type the amount in satoshis using a decimal point as a separator")
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        });
    if !match Confirm::new("Send?").prompt() {
        Ok(b) => b,
        Err(err) => {
            println!("{}", err.to_string().red());
            process::exit(0)
        }
    } {
        return;
    }
    let mut stake = Stake::new(deposit, amount as types::Amount, fee);
    stake.sign(&wallet.keypair);
    println!("Hash: {}", hex::encode(stake.hash()).cyan());
    match post::stake(api, &stake).await {
        Ok(res) => println!(
            "{}",
            if res == "success" {
                res.green()
            } else {
                res.red()
            }
        ),
        Err(err) => println!("{}", err.to_string().red()),
    };
}
fn address(wallet: &Wallet) {
    println!("{}", wallet.address().green());
}
async fn search(api: &str) {
    let search = CustomType::<String>::new("Search:")
        .with_error_message("Please enter a valid address or block hash.")
        .with_help_message("Enter address or block hash.")
        .with_parser(&|x| {
            if address::decode(x).is_ok() || x.len() == 64 {
                return Ok(x.to_string());
            }
            Err(())
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        });
    if address::decode(&search).is_ok() {
        balance(api, &search).await;
    } else if search.len() == 64 {
        match get::block(api, &search).await {
            Ok(block) => {
                println!("{} {}", "Forger".cyan(), address::encode(&block.public_key));
                println!(
                    "{} {}",
                    "Timestamp".cyan(),
                    Local
                        .timestamp(block.timestamp as i64, 0)
                        .format("%H:%M:%S")
                );
                println!(
                    "{} {}",
                    "Transactions".cyan(),
                    block.transactions.len().to_string().yellow()
                );
                for (i, transaction) in block.transactions.iter().enumerate() {
                    println!(
                        "{} {}",
                        format!("#{}", i).magenta(),
                        hex::encode(transaction.hash())
                    )
                }
                println!(
                    "{} {}",
                    "Stakes".cyan(),
                    block.stakes.len().to_string().yellow()
                );
                for (i, stake) in block.stakes.iter().enumerate() {
                    println!(
                        "{} {}",
                        format!("#{}", i).magenta(),
                        hex::encode(stake.hash())
                    )
                }
            }
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
}
fn key(wallet: &Wallet) {
    println!("{}", "Are you being watched?".yellow());
    println!("{}", "Never share your secret key!".yellow());
    println!(
        "{}",
        "Anyone who has it can access your funds from anywhere.".italic()
    );
    println!("{}", "View in private with no cameras around.".italic());
    if match Confirm::new("View secret key?").prompt() {
        Ok(b) => b,
        Err(err) => {
            println!("{}", err.to_string().red());
            process::exit(0)
        }
    } {
        println!("{}", wallet.key().red());
    }
}
fn data(wallet: &Wallet) {
    println!(
        "{}{}{}",
        hex::encode(&wallet.salt).red(),
        hex::encode(&wallet.nonce).red(),
        hex::encode(&wallet.ciphertext).red()
    );
}
fn exit() {
    process::exit(0);
}
