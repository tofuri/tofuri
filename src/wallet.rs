use crate::{address, command, constants::EXTENSION, kdf, key, types, util};
use argon2::password_hash::rand_core::RngCore;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305,
};
use colored::*;
use std::{error::Error, fs::File, io::prelude::*, path::Path, process};
pub struct Wallet {
    pub keypair: types::Keypair,
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
            keypair: util::keygen(),
            salt: vec![],
            nonce: vec![],
            ciphertext: vec![],
        }
    }
    pub fn import(wallet_filename: &str, passphrase: &str) -> Result<Wallet, Box<dyn Error>> {
        if !wallet_filename.is_empty() || !passphrase.is_empty() {
            if !(!wallet_filename.is_empty() && !passphrase.is_empty()) {
                println!(
                    "{}",
                    "To use autodecrypt you must specify both --wallet and --passphrase!".red()
                );
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
        let (filename, wallet) = command::select_wallet()?;
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
    pub fn import_attempt(filename: &str, passphrase: &str) -> Result<Wallet, Box<dyn Error>> {
        let mut path = Wallet::default_path().join(filename);
        path.set_extension(EXTENSION);
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
        let secret_key =
            types::SecretKey::from_bytes(&Wallet::decrypt(salt, nonce, ciphertext, passphrase)?)?;
        let public_key: types::PublicKey = (&secret_key).into();
        Ok(Wallet {
            keypair: types::Keypair {
                secret: secret_key,
                public: public_key,
            },
            salt: salt.to_vec(),
            nonce: nonce.to_vec(),
            ciphertext: ciphertext.to_vec(),
        })
    }
    pub fn export(&mut self, filename: String) -> Result<(), Box<dyn Error>> {
        let (salt, nonce, ciphertext) = Wallet::encrypt(self.keypair.secret.as_bytes())?;
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
        let mut buf = [0; 92];
        file.read_exact(&mut buf)?;
        Ok(buf)
    }
    fn write_all(path: impl AsRef<Path>, buf: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(path)?;
        file.write_all(buf)?;
        Ok(())
    }
    fn default_path() -> &'static Path {
        Path::new("./wallets")
    }
    pub fn address(&self) -> String {
        address::encode(self.keypair.public.as_bytes())
    }
    pub fn key(&self) -> String {
        key::encode(&self.keypair.secret)
    }
    pub fn encrypt(plaintext: &[u8]) -> Result<([u8; 32], [u8; 12], Vec<u8>), Box<dyn Error>> {
        let passphrase = command::new_passphrase();
        let rng = &mut OsRng;
        let mut salt = [0; 32];
        rng.fill_bytes(&mut salt);
        let key = kdf::derive(passphrase.as_bytes(), &salt);
        let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext).unwrap();
        Ok((salt, nonce.into(), ciphertext))
    }
    pub fn decrypt(
        salt: &[u8],
        nonce: &[u8],
        ciphertext: &[u8],
        passphrase: &str,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let passphrase = match passphrase {
            "" => command::passphrase(),
            _ => passphrase.to_string(),
        };
        let key = kdf::derive(passphrase.as_bytes(), salt);
        let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
        match cipher.decrypt(nonce.into(), ciphertext) {
            Ok(plaintext) => Ok(plaintext),
            Err(_) => Err("invalid passphrase".into()),
        }
    }
    pub fn dir() -> Result<Vec<String>, Box<dyn Error>> {
        if !Wallet::default_path().exists() {
            std::fs::create_dir(Wallet::default_path())?;
        }
        let dir = std::fs::read_dir(Wallet::default_path())?;
        let mut filenames: Vec<String> = vec![];
        for entry in dir {
            filenames.push(
                entry?
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        Ok(filenames)
    }
}
