use argon2::{Algorithm, Argon2, ParamsBuilder, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305,
};
use colored::*;
use inquire::{validator::Validation, Password, PasswordDisplayMode, Select};
use pea_core::{constants::EXTENSION, types};
use pea_key::Key;
use std::{
    error::Error,
    fs::{create_dir_all, read_dir, File},
    io::prelude::*,
    path::{Path, PathBuf},
    process,
};
const GENERATE: &str = "Generate new wallet";
const IMPORT: &str = "Import existing wallet";
const INCORRECT: &str = "Incorrect passphrase";
pub type Salt = [u8; 32];
pub type Nonce = [u8; 12];
pub type Ciphertext = [u8; 48];
pub fn argon2_key_derivation(password: &[u8], salt: &[u8; 32]) -> types::Hash {
    let mut builder = ParamsBuilder::new();
    builder.m_cost(1024).unwrap();
    builder.t_cost(1).unwrap();
    builder.p_cost(1).unwrap();
    let params = builder.params().unwrap();
    let ctx = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut bytes = [0; 32];
    ctx.hash_password_into(password, salt, &mut bytes).unwrap();
    bytes
}
fn encrypt(key: &Key) -> Result<(Salt, Nonce, Ciphertext), Box<dyn Error>> {
    let passphrase = inquire_new_passphrase();
    let salt: Salt = rand::random();
    let cipher_key = argon2_key_derivation(passphrase.as_bytes(), &salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&cipher_key)?;
    let nonce: Nonce = rand::random();
    let ciphertext: Ciphertext = cipher
        .encrypt(&nonce.try_into()?, key.secret_key_bytes().as_slice())
        .unwrap()
        .try_into()
        .unwrap();
    Ok((salt, nonce.into(), ciphertext))
}
fn decrypt(salt: &Salt, nonce: &Nonce, ciphertext: &Ciphertext, passphrase: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let passphrase = match passphrase {
        "" => inquire_passphrase(),
        _ => passphrase.to_string(),
    };
    let key = argon2_key_derivation(passphrase.as_bytes(), salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
    match cipher.decrypt(nonce.into(), ciphertext.as_slice()) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => Err("invalid passphrase".into()),
    }
}
fn default_path() -> &'static Path {
    Path::new("./peacash/wallets")
}
fn save(salt: Salt, nonce: Nonce, ciphertext: Ciphertext, filename: &str) -> Result<(), Box<dyn Error>> {
    let mut bytes = [0; 228];
    bytes[0..32].copy_from_slice(&salt);
    bytes[32..44].copy_from_slice(&nonce);
    bytes[44..228].copy_from_slice(&ciphertext);
    let mut path = default_path().join(filename);
    path.set_extension(EXTENSION);
    let mut file = File::create(path)?;
    file.write_all(hex::encode(bytes).as_bytes())?;
    Ok(())
}
fn read_exact(path: impl AsRef<Path>) -> Result<[u8; 92], Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut bytes = [0; 184];
    file.read_exact(&mut bytes)?;
    let vec = hex::decode(bytes).unwrap();
    Ok(vec.try_into().unwrap())
}
pub fn load(filename: &str, passphrase: &str) -> Result<(Salt, Nonce, Ciphertext, Key), Box<dyn Error>> {
    fn attempt(slice: &[u8], passphrase: &str) -> Result<(Salt, Nonce, Ciphertext, Key), Box<dyn Error>> {
        let salt: Salt = slice[0..32].try_into()?;
        let nonce: Nonce = slice[32..44].try_into()?;
        let ciphertext: Ciphertext = slice[44..92].try_into()?;
        let key = Key::from_slice(decrypt(&salt, &nonce, &ciphertext, passphrase)?.as_slice().try_into()?);
        Ok((salt, nonce, ciphertext, key))
    }
    if filename.is_empty() ^ passphrase.is_empty() {
        println!("{}", "To use autodecrypt you must specify both --wallet and --passphrase!".red());
        process::exit(0);
    }
    if !filename.is_empty() && !passphrase.is_empty() {
        return match load(filename, passphrase) {
            Ok(x) => Ok(x),
            Err(_) => {
                println!("{}", INCORRECT.red());
                process::exit(0);
            }
        };
    }
    let filename = inquire_wallet_select()?;
    let bytes = match read_exact(default_path().join(filename)) {
        Ok(x) => x,
        Err(err) => {
            println!("{}", err.to_string().red());
            process::exit(0);
        }
    };
    loop {
        let passphrase = inquire_passphrase();
        if let Ok((salt, nonce, ciphertext, key)) = attempt(&bytes, &passphrase) {
            return Ok((salt, nonce, ciphertext, key));
        } else {
            println!("{}", INCORRECT.red());
        }
    }
}
fn filenames() -> Result<Vec<String>, Box<dyn Error>> {
    let path = default_path();
    if !path.exists() {
        create_dir_all(path)?;
    }
    let mut filenames: Vec<String> = vec![];
    for entry in read_dir(path)? {
        filenames.push(entry?.path().file_name().unwrap().to_string_lossy().into_owned());
    }
    Ok(filenames)
}
fn inquire_wallet_select() -> Result<String, Box<dyn Error>> {
    let mut filenames = filenames()?;
    filenames.push(GENERATE.to_string());
    filenames.push(IMPORT.to_string());
    let name = Select::new(">>", filenames.to_vec()).prompt().unwrap_or_else(|err| {
        println!("{}", err.to_string().red());
        process::exit(0)
    });
    if name.as_str() == GENERATE {
        let name = inquire_wallet_name()?;
        let key = Key::generate();
        let (salt, nonce, ciphertext) = encrypt(&key)?;
        save(salt, nonce, ciphertext, &name)?;
        return Ok(name);
    } else if name.as_str() == IMPORT {
        let key = inquire_wallet_import()?;
        let name = inquire_wallet_name()?;
        let (salt, nonce, ciphertext) = encrypt(&key)?;
        save(salt, nonce, ciphertext, &name)?;
        return Ok(name);
    };
    Ok(name)
}
fn inquire_wallet_name() -> Result<String, Box<dyn Error>> {
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
fn inquire_passphrase() -> String {
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
fn inquire_new_passphrase() -> String {
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
fn inquire_wallet_import() -> Result<Key, Box<dyn Error>> {
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
