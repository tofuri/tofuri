use crate::Wallet;
use chrono::{Local, TimeZone};
use colored::*;
use crossterm::{event, terminal};
use inquire::{Confirm, CustomType, Select};
use pea_address as address;
use pea_amount as amount;
use pea_api::{get, post};
use pea_core::constants::DECIMAL_PRECISION;
use pea_stake::Stake;
use pea_transaction::Transaction;
use std::process;
pub async fn main(wallet: &Wallet, api: &str) {
    match Select::new(
        ">>",
        vec!["Address", "Balance", "Search", "Send", "Height", "Secret", "Encrypted", "Stake", "API", "Exit"],
    )
    .prompt()
    .unwrap_or_else(|err| {
        println!("{}", err.to_string().red());
        process::exit(0)
    }) {
        "Address" => address(wallet),
        "Balance" => balance(api, &wallet.key.public()).await,
        "Search" => search(api).await,
        "Send" => transaction(api, wallet).await,
        "Height" => height(api).await,
        "Secret" => key(wallet),
        "Encrypted" => data(wallet),
        "Stake" => stake(api, wallet).await,
        "API" => info(api).await,
        "Exit" => exit(),
        _ => {}
    }
}
pub fn press_any_key_to_continue() {
    println!("{}", "Press any key to continue...".magenta().italic());
    terminal::enable_raw_mode().unwrap();
    event::read().unwrap();
    terminal::disable_raw_mode().unwrap();
    clear();
}
pub fn clear() {
    print!("\x1B[2J\x1B[1;1H");
}
async fn info(api: &str) {
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
                (balance as f64 / DECIMAL_PRECISION as f64).to_string().yellow(),
                (balance_staked as f64 / DECIMAL_PRECISION as f64).to_string().yellow()
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
        .with_parser(&|x| match address::public::decode(x) {
            Ok(y) => Ok(address::public::encode(&y)),
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
            Ok(f) => Ok(amount::round(&((f * DECIMAL_PRECISION as f64) as u128)) as f64 / DECIMAL_PRECISION as f64),
            Err(_) => Err(()),
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
        * DECIMAL_PRECISION as f64) as u128;
    let fee = CustomType::<u128>::new("Fee:")
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
    let mut transaction = Transaction::new(address::public::decode(&address).unwrap(), amount, fee);
    transaction.sign(&wallet.key);
    println!("Hash: {}", hex::encode(transaction.hash()).cyan());
    match post::transaction(api, &transaction).await {
        Ok(res) => println!("{}", if res == "success" { res.green() } else { res.red() }),
        Err(err) => println!("{}", err.to_string().red()),
    };
}
async fn stake(api: &str, wallet: &Wallet) {
    let deposit = match Select::new(">>", vec!["deposit", "withdraw"]).prompt().unwrap_or_else(|err| {
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
            Ok(f) => Ok(amount::round(&((f * DECIMAL_PRECISION as f64) as u128)) as f64 / DECIMAL_PRECISION as f64),
            Err(_) => Err(()),
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
        * DECIMAL_PRECISION as f64) as u128;
    let fee = CustomType::<u128>::new("Fee:")
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
    let mut stake = Stake::new(deposit, amount, fee);
    stake.sign(&wallet.key);
    println!("Hash: {}", hex::encode(stake.hash()).cyan());
    match post::stake(api, &stake).await {
        Ok(res) => println!("{}", if res == "success" { res.green() } else { res.red() }),
        Err(err) => println!("{}", err.to_string().red()),
    };
}
fn address(wallet: &Wallet) {
    println!("{}", wallet.key.public().green());
}
async fn search(api: &str) {
    let search = CustomType::<String>::new("Search:")
        .with_error_message("Please enter a valid Address, Hash or Number.")
        .with_help_message("Search Blockchain, Transactions, Addresses, Blocks and Stakes")
        .with_parser(&|x| {
            if address::public::decode(x).is_ok() || x.len() == 64 || x.parse::<usize>().is_ok() {
                return Ok(x.to_string());
            }
            Err(())
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        });
    if address::public::decode(&search).is_ok() {
        balance(api, &search).await;
        return;
    } else if search.len() == 64 {
        if let Ok(block) = get::block(api, &search).await {
            print_block(&block, &search);
            return;
        };
        if let Ok(transaction) = get::transaction(api, &search).await {
            println!("{} {}", "Hash".cyan(), search);
            println!("{} {}", "Input PubKey".cyan(), transaction.public_key_input);
            println!("{} {}", "Output PubKey".cyan(), transaction.public_key_output);
            println!("{} {}", "Amount".cyan(), transaction.amount.to_string().yellow());
            println!("{} {}", "Fee".cyan(), transaction.fee.to_string().yellow());
            println!("{} {}", "Timestamp".cyan(), Local.timestamp(transaction.timestamp as i64, 0).format("%H:%M:%S"));
            println!("{} {}", "Signature".cyan(), transaction.signature);
            return;
        };
        if let Ok(stake) = get::stake(api, &search).await {
            println!("{} {}", "Hash".cyan(), search);
            println!("{} {}", "PubKey".cyan(), stake.public_key);
            println!("{} {}", "Amount".cyan(), stake.amount.to_string().yellow());
            println!("{}", if stake.deposit { "Deposit".magenta() } else { "Withdraw".magenta() });
            println!("{} {}", "Fee".cyan(), stake.fee.to_string().yellow());
            println!("{} {}", "Timestamp".cyan(), Local.timestamp(stake.timestamp as i64, 0).format("%H:%M:%S"));
            println!("{} {}", "Signature".cyan(), stake.signature);
            return;
        };
    } else if search.parse::<usize>().is_ok() {
        if let Ok(hash) = get::hash(api, &search.parse::<usize>().unwrap()).await {
            if let Ok(block) = get::block(api, &hash).await {
                print_block(&block, &hash);
                return;
            };
            return;
        };
    }
    println!("{}", "Nothing found".red());
    fn print_block(block: &get::Block, hash: &String) {
        println!("{} {}", "Hash".cyan(), hash);
        println!("{} {}", "PreviousHash".cyan(), block.previous_hash);
        println!("{} {}", "Timestamp".cyan(), Local.timestamp(block.timestamp as i64, 0).format("%H:%M:%S"));
        println!("{} {}", "Forger".cyan(), block.public_key);
        println!("{} {}", "Signature".cyan(), block.signature);
        println!("{} {}", "Transactions".cyan(), block.transactions.len().to_string().yellow());
        for (i, hash) in block.transactions.iter().enumerate() {
            println!("{} {}", format!("#{}", i).magenta(), hash)
        }
        println!("{} {}", "Stakes".cyan(), block.stakes.len().to_string().yellow());
        for (i, hash) in block.stakes.iter().enumerate() {
            println!("{} {}", format!("#{}", i).magenta(), hash)
        }
    }
}
fn key(wallet: &Wallet) {
    println!("{}", "Are you being watched?".yellow());
    println!("{}", "Never share your secret key!".yellow());
    println!("{}", "Anyone who has it can access your funds from anywhere.".italic());
    println!("{}", "View in private with no cameras around.".italic());
    if match Confirm::new("View secret key?").prompt() {
        Ok(b) => b,
        Err(err) => {
            println!("{}", err.to_string().red());
            process::exit(0)
        }
    } {
        println!("{}", wallet.key.secret().red());
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
