use argon2::password_hash::rand_core::RngCore;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305,
};
use pea_core::{constants::EXTENSION, types};
pub mod command;
pub mod kdf;
use colored::*;
use inquire::{validator::Validation, Password, PasswordDisplayMode, Select};
use pea_key::Key;
use std::{
    error::Error,
    fs::File,
    io::prelude::*,
    path::{Path, PathBuf},
    process,
};
type Salt = [u8; 32];
type Nonce = [u8; 12];
type Ciphertext = Vec<u8>;
pub struct Wallet {
    pub key: Key,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}
impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}
impl Wallet {
    pub fn new() -> Wallet {
        Wallet {
            key: Key::generate(),
            salt: vec![],
            nonce: vec![],
            ciphertext: vec![],
        }
    }
    pub fn import(wallet_filename: &str, passphrase: &str) -> Result<Wallet, Box<dyn Error>> {
        if !wallet_filename.is_empty() || !passphrase.is_empty() {
            if !(!wallet_filename.is_empty() && !passphrase.is_empty()) {
                println!("{}", "To use autodecrypt you must specify both --wallet and --passphrase!".red());
                process::exit(0);
            }
            return match Wallet::import_attempt(wallet_filename, passphrase) {
                Ok(w) => Ok(w),
                Err(_) => {
                    println!("{}", "No key available with this passphrase.".red());
                    process::exit(0);
                }
            };
        }
        let (filename, wallet) = Wallet::select_wallet()?;
        if let Some(wallet) = wallet {
            return Ok(wallet);
        }
        let wallet;
        loop {
            if let Ok(w) = Wallet::import_attempt(&filename, passphrase) {
                wallet = w;
                break;
            } else {
                println!("{}", "No key available with this passphrase.".red());
            }
        }
        Ok(wallet)
    }
    fn import_attempt(filename: &str, passphrase: &str) -> Result<Wallet, Box<dyn Error>> {
        let path = Wallet::default_path().join(filename);
        let data = match Wallet::read_exact(path) {
            Ok(data) => data,
            Err(err) => {
                println!("{}", err.to_string().red());
                process::exit(0);
            }
        };
        let salt = &data[..32];
        let nonce = &data[32..44];
        let ciphertext = &data[44..];
        let key = Key::from_slice(Wallet::decrypt(salt, nonce, ciphertext, passphrase)?.as_slice().try_into()?);
        Ok(Wallet {
            key,
            salt: salt.to_vec(),
            nonce: nonce.to_vec(),
            ciphertext: ciphertext.to_vec(),
        })
    }
    fn export(&mut self, filename: String) -> Result<(), Box<dyn Error>> {
        let (salt, nonce, ciphertext) = Wallet::encrypt(&self.key.secret_key_bytes())?;
        self.salt = salt.to_vec();
        self.nonce = nonce.to_vec();
        self.ciphertext = ciphertext.to_vec();
        let mut path = Wallet::default_path().join(filename);
        path.set_extension(EXTENSION);
        Wallet::write_all(path, &[salt.to_vec(), nonce.to_vec(), ciphertext].concat())?;
        Ok(())
    }
    fn read_exact(path: impl AsRef<Path>) -> Result<[u8; 92], Box<dyn Error>> {
        let mut file = File::open(path)?;
        let mut buf = [0; 184];
        file.read_exact(&mut buf)?;
        let vec = hex::decode(buf).unwrap();
        Ok(vec.try_into().unwrap())
    }
    fn write_all(path: impl AsRef<Path>, buf: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(path)?;
        file.write_all(hex::encode(buf).as_bytes())?;
        Ok(())
    }
    fn default_path() -> &'static Path {
        Path::new("./peacash-wallet")
    }
    fn encrypt(plaintext: &[u8]) -> Result<(Salt, Nonce, Ciphertext), Box<dyn Error>> {
        let passphrase = Wallet::new_passphrase();
        let rng = &mut OsRng;
        let mut salt = [0; 32];
        rng.fill_bytes(&mut salt);
        let key = kdf::derive(passphrase.as_bytes(), &salt);
        let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext).unwrap();
        Ok((salt, nonce.into(), ciphertext))
    }
    fn decrypt(salt: &[u8], nonce: &[u8], ciphertext: &[u8], passphrase: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let passphrase = match passphrase {
            "" => Wallet::passphrase(),
            _ => passphrase.to_string(),
        };
        let key = kdf::derive(passphrase.as_bytes(), salt);
        let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
        match cipher.decrypt(nonce.into(), ciphertext) {
            Ok(plaintext) => Ok(plaintext),
            Err(_) => Err("invalid passphrase".into()),
        }
    }
    fn dir() -> Result<Vec<String>, Box<dyn Error>> {
        if !Wallet::default_path().exists() {
            std::fs::create_dir_all(Wallet::default_path())?;
        }
        let dir = std::fs::read_dir(Wallet::default_path())?;
        let mut filenames: Vec<String> = vec![];
        for entry in dir {
            filenames.push(entry?.path().file_name().unwrap().to_string_lossy().into_owned());
        }
        Ok(filenames)
    }
    fn select_wallet() -> Result<(String, Option<Wallet>), Box<dyn Error>> {
        let mut filenames = Wallet::dir()?;
        filenames.push("Generate new wallet".to_string());
        let mut filename = Select::new(">>", filenames.to_vec()).prompt().unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        });
        if filename.as_str() == "Generate new wallet" {
            filename = Wallet::name_wallet()?;
            let mut wallet = Wallet::new();
            wallet.export(filename.clone()).unwrap();
            return Ok((filename, Some(wallet)));
        };
        Ok((filename, None))
    }
    fn name_wallet() -> Result<String, Box<dyn Error>> {
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
    fn new_passphrase() -> String {
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
    fn passphrase() -> String {
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
}
