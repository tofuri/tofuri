use crate::{
    address,
    constants::{DECIMAL_PRECISION, EXTENSION, MAX_STAKE, MIN_STAKE},
    print,
    stake::Stake,
    transaction::Transaction,
    types,
    wallet::Wallet,
};
use colored::*;
use inquire::{validator::Validation, Confirm, CustomType, Password, PasswordDisplayMode, Select};
use std::{
    collections::HashMap,
    error::Error,
    io::{stdin, stdout, Write},
    path::PathBuf,
    process,
};
use termion::{input::TermRead, raw::IntoRawMode};
pub async fn main(wallet: &Wallet, api: &str) -> Result<(), Box<dyn Error>> {
    match Select::new(
        ">>",
        vec![
            "Address",
            "Key",
            "Data",
            "Balance",
            "Height",
            "Transaction",
            "Stake",
            "IP Address",
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
        "Key" => key(wallet),
        "Data" => data(wallet),
        "Balance" => balance(api, &wallet.address()).await?,
        "Height" => height(api).await?,
        "Transaction" => transaction(api, wallet).await?,
        "Stake" => stake(api, wallet).await?,
        "IP Address" => ip().await?,
        "Validator" => validator(api).await?,
        "Exit" => exit(),
        _ => {}
    }
    Ok(())
}
pub fn select_wallet() -> Result<(String, Option<Wallet>), Box<dyn Error>> {
    let mut filenames = Wallet::dir()?;
    filenames.push("Generate new wallet".to_string());
    let mut filename = Select::new(">>", filenames.to_vec())
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        });
    if filename.as_str() == "Generate new wallet" {
        filename = name_wallet()?;
        let mut wallet = Wallet::new();
        wallet.export(filename.clone()).unwrap();
        return Ok((filename, Some(wallet)));
    };
    Ok((filename, None))
}
pub fn name_wallet() -> Result<String, Box<dyn Error>> {
    let filenames = Wallet::dir()?;
    Ok(Password::new("Name:")
        .with_display_toggle_enabled()
        .with_display_mode(PasswordDisplayMode::Full)
        .with_validator(move |input: &str| {
            if input.is_empty() {
                return Ok(Validation::Invalid("A wallet name can't be empty.".into()));
            }
            let mut path = PathBuf::new().join(input);
            path.set_extension(EXTENSION);
            if filenames.contains(&path.file_name().unwrap().to_string_lossy().into_owned()) {
                Ok(Validation::Invalid(
                    "A wallet with that name already exists.".into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        })
        .with_formatter(&|name| name.to_string())
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        }))
}
pub fn press_any_key_to_continue() {
    println!("Press any key to continue...");
    let mut stdout = stdout().into_raw_mode().unwrap();
    stdout.flush().unwrap();
    stdin().events().next();
    print::clear();
}
pub async fn validator(api: &str) -> Result<(), Box<dyn Error>> {
    let info = match reqwest::get(api).await {
        Ok(r) => r,
        Err(err) => return reqwest_err(err),
    }
    .text()
    .await?;
    println!("\n{}\n", info.green());
    Ok(())
}
fn reqwest_err(err: reqwest::Error) -> Result<(), Box<dyn Error>> {
    if err.is_connect() {
        println!("{}", "Connection refused".red());
        Ok(())
    } else {
        Err(Box::new(err))
    }
}
pub async fn balance(api: &str, address: &str) -> Result<(), Box<dyn Error>> {
    let balance = match reqwest::get(format!("{}/balance/{}", api, address)).await {
        Ok(r) => r,
        Err(err) => return reqwest_err(err),
    }
    .json::<types::AxiomAmount>()
    .await?;
    let balance_staked = match reqwest::get(format!("{}/balance_staked/{}", api, address)).await {
        Ok(r) => r,
        Err(err) => return reqwest_err(err),
    }
    .json::<types::AxiomAmount>()
    .await?;
    println!(
        "Account balance: {}, locked: {}.",
        (balance as f64 / DECIMAL_PRECISION as f64)
            .to_string()
            .yellow(),
        (balance_staked as f64 / DECIMAL_PRECISION as f64)
            .to_string()
            .yellow()
    );
    Ok(())
}
pub async fn height(api: &str) -> Result<(), Box<dyn Error>> {
    let height = match reqwest::get(format!("{}/height", api)).await {
        Ok(r) => r,
        Err(err) => return reqwest_err(err),
    }
    .json::<types::Height>()
    .await?;
    println!("Latest block height is {}.", height.to_string().yellow());
    Ok(())
}
pub async fn transaction(api: &str, wallet: &Wallet) -> Result<(), Box<dyn Error>> {
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
        .with_formatter(&|i| format!("{:.9} axiom", i)) // DECIMAL_PRECISION
        .with_error_message("Please type a valid number")
        .with_help_message("Type the amount in axiom using a decimal point as a separator")
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
        * DECIMAL_PRECISION as f64) as types::AxiomAmount;
    let fee = CustomType::<types::AxiomAmount>::new("Fee:")
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
        return Ok(());
    }
    let mut transaction = Transaction::new(address::decode(&address)?, amount, fee);
    transaction.sign(&wallet.keypair);
    let client = reqwest::Client::new();
    let res: usize = match client
        .post(format!("{}/transaction", api))
        .body(hex::encode(bincode::serialize(&transaction)?))
        .send()
        .await
    {
        Ok(r) => r,
        Err(err) => return reqwest_err(err),
    }
    .json()
    .await?;
    println!(
        "{} {}",
        if res == 1 {
            "Transaction successfully sent!".green()
        } else if res == 0 {
            "Transaction failed to send!".red()
        } else {
            "Unexpected status".cyan()
        },
        hex::encode(&transaction.hash())
    );
    Ok(())
}
pub async fn stake(api: &str, wallet: &Wallet) -> Result<(), Box<dyn Error>> {
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
        .with_formatter(&|i| format!("{:.9} axiom", i)) // DECIMAL_PRECISION
        .with_parser(&|x| {
            let amount = match x.parse::<f64>() {
                Ok(a) => a,
                Err(_) => return Err(()),
            };
            if amount * DECIMAL_PRECISION as f64 >= MIN_STAKE as f64
                && amount * DECIMAL_PRECISION as f64 <= MAX_STAKE as f64
            {
                Ok(amount)
            } else {
                Err(())
            }
        })
        .with_error_message("Please type a valid number (1 - 100)")
        .with_help_message("Type the amount in axiom using a decimal point as a separator")
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
        * DECIMAL_PRECISION as f64) as types::AxiomAmount;
    let fee = CustomType::<types::AxiomAmount>::new("Fee:")
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
        return Ok(());
    }
    let mut stake = Stake::new(deposit, amount as types::AxiomAmount, fee);
    stake.sign(&wallet.keypair);
    let client = reqwest::Client::new();
    let res: usize = match client
        .post(format!("{}/stake", api))
        .body(hex::encode(bincode::serialize(&stake)?))
        .send()
        .await
    {
        Ok(r) => r,
        Err(err) => return reqwest_err(err),
    }
    .json()
    .await?;
    println!(
        "{} {}",
        if res == 1 {
            "Stake successfully sent!".green()
        } else if res == 0 {
            "Stake failed to send!".red()
        } else {
            "Unexpected status".cyan()
        },
        hex::encode(&stake.hash())
    );
    Ok(())
}
pub async fn ip() -> Result<(), Box<dyn Error>> {
    let resp = match reqwest::get("https://httpbin.org/ip").await {
        Ok(r) => r,
        Err(err) => return reqwest_err(err),
    }
    .json::<HashMap<String, String>>()
    .await?;
    if let Some(origin) = resp.get("origin") {
        println!("{}", origin.yellow());
    }
    Ok(())
}
pub fn address(wallet: &Wallet) {
    println!("{}", wallet.address().green());
}
pub fn key(wallet: &Wallet) {
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
pub fn data(wallet: &Wallet) {
    println!(
        "{}{}{}",
        hex::encode(&wallet.salt).red(),
        hex::encode(&wallet.nonce).red(),
        hex::encode(&wallet.ciphertext).red()
    );
}
pub fn exit() {
    process::exit(0);
}
pub fn passphrase() -> String {
    Password::new("Enter passphrase:")
        .with_display_toggle_enabled()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_formatter(&|_| String::from("Decrypting..."))
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
}
pub fn new_passphrase() -> String {
    let passphrase = Password::new("New passphrase:")
        .with_display_toggle_enabled()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_validator(move |input: &str| {
            if input.is_empty() {
                Ok(Validation::Invalid("No passphrase isn't allowed.".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .with_formatter(&|input| {
            let entropy = zxcvbn::zxcvbn(input, &[]).unwrap();
            format!(
                "{}. Cracked after {} at 10 guesses per second.",
                match entropy.score() {
                    0 => "Extremely weak",
                    1 => "Very weak",
                    2 => "Weak",
                    3 => "Strong",
                    4 => "Very strong",
                    _ => "",
                },
                entropy.crack_times().online_no_throttling_10_per_second(),
            )
        })
        .with_help_message("It is recommended to generate a new one only for this purpose")
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        });
    Password::new("Confirm new passphrase:")
        .with_display_toggle_enabled()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_validator(move |input: &str| {
            if passphrase != input {
                Ok(Validation::Invalid("Passphrase does not match.".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .with_formatter(&|_| String::from("Encrypting..."))
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
}
