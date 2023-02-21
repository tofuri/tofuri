use crate::inquire;
use crate::inquire::GENERATE;
use crate::inquire::IMPORT;
use ::inquire::Confirm;
use ::inquire::Select;
use argon2::Algorithm;
use argon2::Argon2;
use argon2::ParamsBuilder;
use argon2::Version;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::aead::KeyInit;
use chacha20poly1305::ChaCha20Poly1305;
use colored::*;
use crossterm::event;
use crossterm::terminal;
use pea_address::address;
use pea_address::secret;
use pea_api_core::Block;
use pea_api_core::Root;
use pea_api_core::Stake;
use pea_api_core::Transaction;
use pea_core::*;
use pea_key::Key;
use pea_stake::StakeA;
use pea_transaction::TransactionA;
use std::error::Error;
use std::fs::create_dir_all;
use std::fs::read_dir;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process;
const INCORRECT: &str = "Incorrect passphrase";
pub type Salt = [u8; 32];
pub type Nonce = [u8; 12];
pub type Ciphertext = [u8; 48];
pub struct Options {
    pub api: String,
}
pub struct Wallet {
    key: Option<Key>,
    salt: Salt,
    nonce: Nonce,
    ciphertext: Ciphertext,
    api: String,
}
impl Wallet {
    pub fn new(options: Options) -> Wallet {
        Wallet {
            key: None,
            salt: [0; 32],
            nonce: [0; 12],
            ciphertext: [0; 48],
            api: options.api,
        }
    }
    pub async fn select(&mut self) -> bool {
        let mut vec = vec!["Wallet", "Search", "Height", "API", "Exit"];
        if self.key.is_some() {
            let mut v = vec!["Address", "Balance", "Send", "Stake", "Secret", "Hex"];
            v.append(&mut vec);
            vec = v;
        };
        match Select::new(">>", vec).prompt().unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        }) {
            "Wallet" => {
                self.decrypt();
                false
            }
            "Search" => {
                if let Err(err) = self.search().await {
                    println!("{}", err.to_string().red());
                }
                true
            }
            "Height" => {
                if let Err(err) = self.height().await {
                    println!("{}", err.to_string().red());
                }
                true
            }
            "API" => {
                if let Err(err) = self.api().await {
                    println!("{}", err.to_string().red());
                }
                true
            }
            "Address" => {
                self.address();
                true
            }
            "Balance" => {
                if let Err(err) = self.balance().await {
                    println!("{}", err.to_string().red());
                }
                true
            }
            "Send" => {
                if let Err(err) = self.transaction().await {
                    println!("{}", err.to_string().red());
                }
                true
            }
            "Stake" => {
                if let Err(err) = self.stake().await {
                    println!("{}", err.to_string().red());
                }
                true
            }
            "Secret" => {
                self.key();
                true
            }
            "Hex" => {
                self.data();
                true
            }
            _ => {
                process::exit(0);
            }
        }
    }
    fn decrypt(&mut self) {
        let (salt, nonce, ciphertext, key) = load("", "").unwrap();
        self.salt = salt;
        self.nonce = nonce;
        self.ciphertext = ciphertext;
        self.key = Some(key);
    }
    async fn api(&self) -> Result<(), Box<dyn Error>> {
        let root: Root = reqwest::get(&self.api).await?.json().await?;
        println!("{:#?}", root);
        Ok(())
    }
    async fn balance(&self) -> Result<(), Box<dyn Error>> {
        let address = address::encode(&self.key.as_ref().unwrap().address_bytes());
        let balance: String = reqwest::get(format!("{}/balance/{}", self.api, address)).await?.json().await?;
        let staked: String = reqwest::get(format!("{}/staked/{}", self.api, address)).await?.json().await?;
        println!("Account balance: {}, staked: {}", balance.to_string().yellow(), staked.to_string().yellow());
        Ok(())
    }
    async fn height(&self) -> Result<(), Box<dyn Error>> {
        let height: usize = reqwest::get(format!("{}/height", self.api)).await?.json().await?;
        println!("Latest block height is {}.", height.to_string().yellow());
        Ok(())
    }
    async fn transaction(&self) -> Result<(), Box<dyn Error>> {
        let address = inquire::address();
        let amount = inquire::amount();
        let fee = inquire::fee();
        if !match Confirm::new("Send?").prompt() {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                process::exit(0)
            }
        } {
            return Ok(());
        }
        let transaction_a = TransactionA::sign(
            address::decode(&address).unwrap(),
            amount,
            fee,
            pea_util::timestamp(),
            self.key.as_ref().unwrap(),
        )
        .unwrap();
        println!("Hash: {}", hex::encode(transaction_a.hash).cyan());
        let res: String = reqwest::Client::new()
            .post(format!("{}/transaction", self.api))
            .json(&pea_api_util::transaction(&transaction_a))
            .send()
            .await?
            .json()
            .await?;
        println!("{}", if res == "success" { res.green() } else { res.red() });
        Ok(())
    }
    async fn stake(&self) -> Result<(), Box<dyn Error>> {
        let deposit = inquire::deposit();
        let amount = inquire::amount();
        let fee = inquire::fee();
        let send = inquire::send();
        if !send {
            return Ok(());
        }
        let stake_a = StakeA::sign(deposit, amount, fee, pea_util::timestamp(), self.key.as_ref().unwrap()).unwrap();
        println!("Hash: {}", hex::encode(stake_a.hash).cyan());
        let res: String = reqwest::Client::new()
            .post(format!("{}/stake", self.api))
            .json(&pea_api_util::stake(&stake_a))
            .send()
            .await?
            .json()
            .await?;
        println!("{}", if res == "success" { res.green() } else { res.red() });
        Ok(())
    }
    async fn search(&self) -> Result<(), Box<dyn Error>> {
        let search = inquire::search();
        if address::decode(&search).is_ok() {
            let balance: String = reqwest::get(format!("{}/balance/{}", self.api, search)).await?.json().await?;
            let staked: String = reqwest::get(format!("{}/staked/{}", self.api, search)).await?.json().await?;
            println!(
                "Address found\nAccount balance: {}, staked: {}",
                balance.to_string().yellow(),
                staked.to_string().yellow()
            );
            return Ok(());
        } else if search.len() == 64 {
            if let Ok(res) = reqwest::get(format!("{}/block/{}", self.api, search)).await {
                let block: Block = res.json().await?;
                println!("Block found\n{block:?}");
                return Ok(());
            }
            if let Ok(res) = reqwest::get(format!("{}/transaction/{}", self.api, search)).await {
                let transaction: Transaction = res.json().await?;
                println!("Transaction found\n{transaction:?}");
                return Ok(());
            }
            if let Ok(res) = reqwest::get(format!("{}/stake/{}", self.api, search)).await {
                let stake: Stake = res.json().await?;
                println!("Stake found\n{stake:?}");
                return Ok(());
            }
        } else if search.parse::<usize>().is_ok() {
            if let Ok(res) = reqwest::get(format!("{}/hash/{}", self.api, search)).await {
                let hash: String = res.json().await?;
                if let Ok(res) = reqwest::get(format!("{}/block/{}", self.api, hash)).await {
                    let block: Block = res.json().await?;
                    println!("Block found\n{block:?}");
                    return Ok(());
                }
                return Ok(());
            }
        }
        println!("{}", "Nothing found".red());
        Ok(())
    }
    fn address(&self) {
        println!("{}", address::encode(&self.key.as_ref().unwrap().address_bytes()).green());
    }
    fn key(&self) {
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
            println!("{}", secret::encode(&self.key.as_ref().unwrap().secret_key_bytes()).red());
        }
    }
    fn data(&self) {
        println!(
            "{}{}{}",
            hex::encode(self.salt).red(),
            hex::encode(self.nonce).red(),
            hex::encode(self.ciphertext).red()
        );
    }
}
pub fn argon2_key_derivation(password: &[u8], salt: &[u8; 32]) -> Hash {
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
    Ok((salt, nonce, ciphertext))
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
pub fn save(filename: &str, key: &Key) -> Result<(), Box<dyn Error>> {
    let (salt, nonce, ciphertext) = encrypt(key)?;
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
        let key = Key::from_slice(decrypt(&salt, &nonce, &ciphertext, passphrase)?.as_slice().try_into()?)?;
        Ok((salt, nonce, ciphertext, key))
    }
    if filename.is_empty() ^ passphrase.is_empty() {
        println!("{}", "To use autodecrypt you must specify both --wallet and --passphrase".red());
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
    let mut filename = crate::inquire::select()?;
    if filename.as_str() == *GENERATE {
        let key = Key::generate();
        if !inquire::save() {
            return Ok(([0; 32], [0; 12], [0; 48], key));
        }
        filename = inquire::name()?;
        save(&filename, &key)?;
    } else if filename.as_str() == *IMPORT {
        let key = inquire::import()?;
        if !inquire::save() {
            return Ok(([0; 32], [0; 12], [0; 48], key));
        }
        save(&filename, &key)?;
    };
    let mut path = default_path().join(filename);
    path.set_extension(EXTENSION);
    clear();
    println!("{}", path.to_string_lossy().green());
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
