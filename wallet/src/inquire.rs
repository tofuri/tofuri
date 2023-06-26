use address::public;
use colored::*;
use inquire::validator::Validation;
use inquire::Confirm;
use inquire::CustomType;
use inquire::Password;
use inquire::PasswordDisplayMode;
use inquire::Select;
use inquire::Text;
use key::Key;
use key_store::EXTENSION;
use lazy_static::lazy_static;
use std::error::Error;
use std::path::PathBuf;
use vint::floor;
use vint::Vint;
lazy_static! {
    pub static ref GENERATE: String = "Generate".green().to_string();
    pub static ref IMPORT: String = "Import".magenta().to_string();
}
pub fn select() -> Result<String, Box<dyn Error>> {
    let mut filenames = key_store::filenames();
    filenames.push(GENERATE.to_string());
    filenames.push(IMPORT.to_string());
    Ok(Select::new(">>", filenames.to_vec()).prompt()?)
}
pub fn name_new() -> Result<String, Box<dyn Error>> {
    let filenames = key_store::filenames();
    Ok(Text::new("Name:")
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
        .prompt()?)
}
pub fn save_new() -> Result<bool, Box<dyn Error>> {
    Ok(Confirm::new("Save to disk?").prompt()?)
}
pub fn pwd_new() -> Result<String, Box<dyn Error>> {
    Ok(Password::new("New passphrase:")
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
        .prompt()?)
}
pub fn pwd() -> Result<String, Box<dyn Error>> {
    Ok(Password::new("Enter passphrase:")
        .without_confirmation()
        .with_display_toggle_enabled()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_formatter(&|_| String::from("Decrypting..."))
        .prompt()?)
}
pub fn import_new() -> Result<Key, Box<dyn Error>> {
    let secret = Password::new("Enter secret key:")
        .without_confirmation()
        .with_display_toggle_enabled()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_validator(move |input: &str| match address::secret::decode(input) {
            Ok(_) => Ok(Validation::Valid),
            Err(_) => Ok(Validation::Invalid("Invalid secret key.".into())),
        })
        .with_help_message("Enter a valid secret key (SECRETx...)")
        .prompt()?;
    Ok(Key::from_slice(&address::secret::decode(&secret).unwrap()).unwrap())
}
pub fn confirm_send() -> Result<bool, Box<dyn Error>> {
    Ok(Confirm::new("Send?").prompt()?)
}
pub fn search() -> Result<String, Box<dyn Error>> {
    Ok(CustomType::<String>::new("Search:")
        .with_error_message("Please enter a valid Address, [u8; 32] or Number.")
        .with_help_message("Search Blockchain, Transactions, Addresses, Blocks and Stakes")
        .with_parser(&|input| {
            if public::decode(input).is_ok() || input.len() == 64 || input.parse::<usize>().is_ok()
            {
                return Ok(input.to_string());
            }
            Err(())
        })
        .prompt()?)
}
pub fn address() -> Result<String, Box<dyn Error>> {
    Ok(CustomType::<String>::new("Address:")
        .with_error_message("Please enter a valid address")
        .with_help_message("Type the hex encoded address with 0x as prefix")
        .with_parser(&|input| match public::decode(input) {
            Ok(address_bytes) => Ok(public::encode(&address_bytes)),
            Err(_) => Err(()),
        })
        .prompt()?)
}
pub fn amount() -> Result<u128, Box<dyn Error>> {
    const COIN: u128 = 10_u128.pow(18);
    Ok((CustomType::<f64>::new("Amount:")
        .with_formatter(&|i| format!("{i:.18} tofuri"))
        .with_error_message("Please type a valid number")
        .with_help_message("Type the amount to send using a decimal point as a separator")
        .with_parser(&|input| match input.parse::<f64>() {
            Ok(amount) => Ok(floor!((amount * COIN as f64), 4) as f64 / COIN as f64),
            Err(_) => Err(()),
        })
        .prompt()?
        * COIN as f64) as u128)
}
pub fn fee() -> Result<u128, Box<dyn Error>> {
    Ok(CustomType::<u128>::new("Fee:")
        .with_formatter(&|i| format!("{} {}", i, if i == 1 { "satoshi" } else { "satoshis" }))
        .with_error_message("Please type a valid number")
        .with_help_message("Type the fee to use in satoshis")
        .with_parser(&|input| match input.parse::<u128>() {
            Ok(fee) => Ok(floor!(fee, 4)),
            Err(_) => Err(()),
        })
        .prompt()?)
}
pub fn deposit() -> Result<bool, Box<dyn Error>> {
    Ok(
        match Select::new(">>", vec!["deposit", "withdraw"]).prompt()? {
            "deposit" => true,
            "withdraw" => false,
            _ => unreachable!(),
        },
    )
}
