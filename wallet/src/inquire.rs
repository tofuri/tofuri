use crate::util::{encrypt, filenames, save};
use colored::*;
use inquire::{validator::Validation, Confirm, CustomType, Password, PasswordDisplayMode, Select};
use pea_address::address;
use pea_core::*;
use pea_key::Key;
use std::{error::Error, path::PathBuf, process};
const GENERATE: &str = "Generate new wallet";
const IMPORT: &str = "Import wallet";
pub fn wallet_select() -> Result<String, Box<dyn Error>> {
    let mut filenames = filenames()?;
    filenames.push(GENERATE.to_string());
    filenames.push(IMPORT.to_string());
    let filename = Select::new(">>", filenames.to_vec()).prompt().unwrap_or_else(|err| {
        println!("{}", err.to_string().red());
        process::exit(0)
    });
    if filename.as_str() == GENERATE {
        let filename = wallet_name()?;
        let key = Key::generate();
        let (salt, nonce, ciphertext) = encrypt(&key)?;
        save(salt, nonce, ciphertext, &filename)?;
        return Ok(filename);
    } else if filename.as_str() == IMPORT {
        let key = wallet_import()?;
        let filename = wallet_name()?;
        let (salt, nonce, ciphertext) = encrypt(&key)?;
        save(salt, nonce, ciphertext, &filename)?;
        return Ok(filename);
    };
    Ok(filename)
}
pub fn wallet_name() -> Result<String, Box<dyn Error>> {
    let filenames = filenames()?;
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
                Ok(Validation::Invalid("A wallet with that name already exists.".into()))
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
pub fn wallet_import() -> Result<Key, Box<dyn Error>> {
    let secret = Password::new("Enter secret key:")
        .with_display_toggle_enabled()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_validator(move |input: &str| match pea_address::secret::decode(input) {
            Ok(_) => Ok(Validation::Valid),
            Err(_) => Ok(Validation::Invalid("Invalid secret key.".into())),
        })
        .with_help_message("Enter a valid secret key (SECRETx...)")
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        });
    Ok(Key::from_slice(&pea_address::secret::decode(&secret)?))
}
pub fn send() -> bool {
    match Confirm::new("Send?").prompt() {
        Ok(b) => b,
        Err(err) => {
            println!("{}", err.to_string().red());
            process::exit(0)
        }
    }
}
pub fn search() -> String {
    CustomType::<String>::new("Search:")
        .with_error_message("Please enter a valid Address, Hash or Number.")
        .with_help_message("Search Blockchain, Transactions, Addresses, Blocks and Stakes")
        .with_parser(&|x| {
            if address::decode(x).is_ok() || x.len() == 64 || x.parse::<usize>().is_ok() {
                return Ok(x.to_string());
            }
            Err(())
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
}
pub fn address() -> String {
    CustomType::<String>::new("Address:")
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
        })
}
pub fn amount() -> u128 {
    (CustomType::<f64>::new("Amount:")
        .with_formatter(&|i| format!("{:.18} pea", i))
        .with_error_message("Please type a valid number")
        .with_help_message("Type the amount to send using a decimal point as a separator")
        .with_parser(&|x| match x.parse::<f64>() {
            Ok(f) => Ok(pea_int::floor((f * COIN as f64) as u128) as f64 / COIN as f64),
            Err(_) => Err(()),
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
        * COIN as f64) as u128
}
pub fn fee() -> u128 {
    CustomType::<u128>::new("Fee:")
        .with_formatter(&|i| format!("{} {}", i, if i == 1 { "satoshi" } else { "satoshis" }))
        .with_error_message("Please type a valid number")
        .with_help_message("Type the fee to use in satoshis")
        .with_parser(&|x| match x.parse::<u128>() {
            Ok(u) => Ok(pea_int::floor(u)),
            Err(_) => Err(()),
        })
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        })
}
pub fn deposit() -> bool {
    match Select::new(">>", vec!["deposit", "withdraw"]).prompt().unwrap_or_else(|err| {
        println!("{}", err.to_string().red());
        process::exit(0)
    }) {
        "deposit" => true,
        "withdraw" => false,
        _ => false,
    }
}
