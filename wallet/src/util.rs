use argon2::{Algorithm, Argon2, ParamsBuilder, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305,
};
use colored::*;
use crossterm::{event, terminal};
use pea_core::{constants::EXTENSION, types};
use pea_key::Key;
use std::{
    error::Error,
    fs::{create_dir_all, read_dir, File},
    io::prelude::*,
    path::Path,
    process,
};
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
pub fn encrypt(key: &Key) -> Result<(Salt, Nonce, Ciphertext), Box<dyn Error>> {
    let passphrase = crate::inquire::new_passphrase();
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
pub fn decrypt(salt: &Salt, nonce: &Nonce, ciphertext: &Ciphertext, passphrase: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let passphrase = match passphrase {
        "" => crate::inquire::passphrase(),
        _ => passphrase.to_string(),
    };
    let key = argon2_key_derivation(passphrase.as_bytes(), salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
    match cipher.decrypt(nonce.into(), ciphertext.as_slice()) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => Err("invalid passphrase".into()),
    }
}
pub fn default_path() -> &'static Path {
    Path::new("./peacash-wallet")
}
pub fn save(salt: Salt, nonce: Nonce, ciphertext: Ciphertext, filename: &str) -> Result<(), Box<dyn Error>> {
    let mut bytes = [0; 92];
    bytes[0..32].copy_from_slice(&salt);
    bytes[32..44].copy_from_slice(&nonce);
    bytes[44..92].copy_from_slice(&ciphertext);
    let mut path = default_path().join(filename);
    path.set_extension(EXTENSION);
    let mut file = File::create(path)?;
    file.write_all(hex::encode(bytes).as_bytes())?;
    Ok(())
}
pub fn load(filename: &str, passphrase: &str) -> Result<(Salt, Nonce, Ciphertext, Key), Box<dyn Error>> {
    fn read_exact(path: impl AsRef<Path>) -> Result<[u8; 92], Box<dyn Error>> {
        let mut file = File::open(path)?;
        let mut bytes = [0; 184];
        file.read_exact(&mut bytes)?;
        let vec = hex::decode(bytes).unwrap();
        Ok(vec.try_into().unwrap())
    }
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
        let mut path = default_path().join(filename);
        path.set_extension(EXTENSION);
        let bytes = match read_exact(path) {
            Ok(x) => x,
            Err(err) => {
                println!("{}", err.to_string().red());
                process::exit(0);
            }
        };
        return match attempt(&bytes, passphrase) {
            Ok(x) => Ok(x),
            Err(_) => {
                println!("{}", INCORRECT.red());
                process::exit(0);
            }
        };
    }
    let filename = crate::inquire::wallet_select()?;
    let mut path = default_path().join(filename);
    path.set_extension(EXTENSION);
    let bytes = match read_exact(path) {
        Ok(x) => x,
        Err(err) => {
            println!("{}", err.to_string().red());
            process::exit(0);
        }
    };
    loop {
        let passphrase = crate::inquire::passphrase();
        if let Ok((salt, nonce, ciphertext, key)) = attempt(&bytes, &passphrase) {
            return Ok((salt, nonce, ciphertext, key));
        } else {
            println!("{}", INCORRECT.red());
        }
    }
}
pub fn filenames() -> Result<Vec<String>, Box<dyn Error>> {
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
pub fn press_any_key_to_continue() {
    println!("{}", "Press any key to continue...".magenta().italic());
    terminal::enable_raw_mode().unwrap();
    event::read().unwrap();
    terminal::disable_raw_mode().unwrap();
}
pub fn clear() {
    print!("\x1B[2J\x1B[1;1H");
}
