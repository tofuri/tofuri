pub mod inquire;
pub mod util;
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
use clap::Parser;
use colored::*;
use crossterm::event;
use crossterm::terminal;
use reqwest::Client;
use reqwest::Url;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process;
use tofuri_address::address;
use tofuri_address::secret;
use tofuri_api_core::Block;
use tofuri_api_core::Root;
use tofuri_api_core::Stake;
use tofuri_api_core::Transaction;
use tofuri_core::*;
use tofuri_key::Key;
use tofuri_stake::StakeA;
use tofuri_transaction::TransactionA;
pub const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CARGO_PKG_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const INCORRECT: &str = "Incorrect passphrase";
pub type Salt = [u8; 32];
pub type Nonce = [u8; 12];
pub type Ciphertext = [u8; 48];
#[derive(Debug)]
pub enum Error {
    Key(tofuri_key::Error),
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Inquire(inquire::Error),
    InvalidPassphrase,
}
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, env = "API", default_value = "http://localhost:2022/")]
    pub api: Url,
}
#[derive(Debug, Clone)]
pub struct Wallet {
    key: Option<Key>,
    salt: Salt,
    nonce: Nonce,
    ciphertext: Ciphertext,
    args: Args,
    client: Client,
}
impl Wallet {
    pub fn new(args: Args) -> Wallet {
        Wallet {
            key: None,
            salt: [0; 32],
            nonce: [0; 12],
            ciphertext: [0; 48],
            args,
            client: Client::default(),
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
                    println!("{:?}", err);
                }
                true
            }
            "Height" => {
                if let Err(err) = self.height().await {
                    println!("{:?}", err);
                }
                true
            }
            "API" => {
                if let Err(err) = self.api().await {
                    println!("{:?}", err);
                }
                true
            }
            "Address" => {
                self.address();
                true
            }
            "Balance" => {
                if let Err(err) = self.balance().await {
                    println!("{:?}", err);
                }
                true
            }
            "Send" => {
                if let Err(err) = self.transaction().await {
                    println!("{:?}", err);
                }
                true
            }
            "Stake" => {
                if let Err(err) = self.stake().await {
                    println!("{:?}", err);
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
        let (salt, nonce, ciphertext, key) = load().unwrap();
        self.salt = salt;
        self.nonce = nonce;
        self.ciphertext = ciphertext;
        self.key = Some(key);
    }
    async fn api(&self) -> Result<(), Error> {
        let root: Root = self
            .client
            .get(self.args.api.to_string())
            .send()
            .await
            .map_err(Error::Reqwest)?
            .json()
            .await
            .map_err(Error::Reqwest)?;
        println!("{root:#?}");
        Ok(())
    }
    async fn balance(&self) -> Result<(), Error> {
        let address = address::encode(&self.key.as_ref().unwrap().address_bytes());
        let balance: String = self
            .client
            .get(format!("{}balance/{}", self.args.api.to_string(), address))
            .send()
            .await
            .map_err(Error::Reqwest)?
            .json()
            .await
            .map_err(Error::Reqwest)?;
        let staked: String = self
            .client
            .get(format!("{}staked/{}", self.args.api.to_string(), address))
            .send()
            .await
            .map_err(Error::Reqwest)?
            .json()
            .await
            .map_err(Error::Reqwest)?;
        println!(
            "Account balance: {}, staked: {}",
            balance.to_string().yellow(),
            staked.yellow()
        );
        Ok(())
    }
    async fn height(&self) -> Result<(), Error> {
        let height: usize = self
            .client
            .get(format!("{}height", self.args.api.to_string()))
            .send()
            .await
            .map_err(Error::Reqwest)?
            .json()
            .await
            .map_err(Error::Reqwest)?;
        println!("Latest block height is {}.", height.to_string().yellow());
        Ok(())
    }
    async fn transaction(&self) -> Result<(), Error> {
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
            tofuri_util::timestamp(),
            self.key.as_ref().unwrap(),
        )
        .unwrap();
        println!("Hash: {}", hex::encode(transaction_a.hash).cyan());
        let res: String = self
            .client
            .post(format!("{}transaction", self.args.api.to_string()))
            .json(&tofuri_api_util::transaction(&transaction_a))
            .send()
            .await
            .map_err(Error::Reqwest)?
            .json()
            .await
            .map_err(Error::Reqwest)?;
        println!(
            "{}",
            if res == "success" {
                res.green()
            } else {
                res.red()
            }
        );
        Ok(())
    }
    async fn stake(&self) -> Result<(), Error> {
        let deposit = inquire::deposit();
        let amount = inquire::amount();
        let fee = inquire::fee();
        let send = inquire::send();
        if !send {
            return Ok(());
        }
        let stake_a = StakeA::sign(
            deposit,
            amount,
            fee,
            tofuri_util::timestamp(),
            self.key.as_ref().unwrap(),
        )
        .unwrap();
        println!("Hash: {}", hex::encode(stake_a.hash).cyan());
        let res: String = self
            .client
            .post(format!("{}stake", self.args.api.to_string()))
            .json(&tofuri_api_util::stake(&stake_a))
            .send()
            .await
            .map_err(Error::Reqwest)?
            .json()
            .await
            .map_err(Error::Reqwest)?;
        println!(
            "{}",
            if res == "success" {
                res.green()
            } else {
                res.red()
            }
        );
        Ok(())
    }
    async fn search(&self) -> Result<(), Error> {
        let search = inquire::search();
        if address::decode(&search).is_ok() {
            let balance: String = self
                .client
                .get(format!("{}balance/{}", self.args.api.to_string(), search))
                .send()
                .await
                .map_err(Error::Reqwest)?
                .json()
                .await
                .map_err(Error::Reqwest)?;
            let staked: String = self
                .client
                .get(format!("{}staked/{}", self.args.api.to_string(), search))
                .send()
                .await
                .map_err(Error::Reqwest)?
                .json()
                .await
                .map_err(Error::Reqwest)?;
            println!(
                "Address found\nAccount balance: {}, staked: {}",
                balance.to_string().yellow(),
                staked.yellow()
            );
            return Ok(());
        } else if search.len() == 64 {
            if let Ok(res) = self
                .client
                .get(format!("{}block/{}", self.args.api.to_string(), search))
                .send()
                .await
            {
                let block: Block = res.json().await.map_err(Error::Reqwest)?;
                println!("Block found\n{block:?}");
                return Ok(());
            }
            if let Ok(res) = self
                .client
                .get(format!(
                    "{}/transaction/{}",
                    self.args.api.to_string(),
                    search
                ))
                .send()
                .await
            {
                let transaction: Transaction = res.json().await.map_err(Error::Reqwest)?;
                println!("Transaction found\n{transaction:?}");
                return Ok(());
            }
            if let Ok(res) = self
                .client
                .get(format!("{}stake/{}", self.args.api.to_string(), search))
                .send()
                .await
            {
                let stake: Stake = res.json().await.map_err(Error::Reqwest)?;
                println!("Stake found\n{stake:?}");
                return Ok(());
            }
        } else if search.parse::<usize>().is_ok() {
            if let Ok(res) = self
                .client
                .get(format!("{}hash/{}", self.args.api.to_string(), search))
                .send()
                .await
            {
                let hash: String = res.json().await.map_err(Error::Reqwest)?;
                if let Ok(res) = self
                    .client
                    .get(format!("{}block/{}", self.args.api.to_string(), hash))
                    .send()
                    .await
                {
                    let block: Block = res.json().await.map_err(Error::Reqwest)?;
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
        println!(
            "{}",
            address::encode(&self.key.as_ref().unwrap().address_bytes()).green()
        );
    }
    fn key(&self) {
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
            println!(
                "{}",
                secret::encode(&self.key.as_ref().unwrap().secret_key_bytes()).red()
            );
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
    let mut params_builder = ParamsBuilder::new();
    params_builder.m_cost(1024);
    params_builder.t_cost(1);
    params_builder.p_cost(1);
    let params = params_builder.build().unwrap();
    let ctx = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut bytes = [0; 32];
    ctx.hash_password_into(password, salt, &mut bytes).unwrap();
    bytes
}
pub fn encrypt(key: &Key) -> Result<(Salt, Nonce, Ciphertext), Error> {
    let passphrase = crate::inquire::new_passphrase();
    let salt: Salt = rand::random();
    let cipher_key = argon2_key_derivation(passphrase.as_bytes(), &salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&cipher_key).unwrap();
    let nonce: Nonce = rand::random();
    let ciphertext: Ciphertext = cipher
        .encrypt(
            &nonce.try_into().unwrap(),
            key.secret_key_bytes().as_slice(),
        )
        .unwrap()
        .try_into()
        .unwrap();
    Ok((salt, nonce, ciphertext))
}
pub fn decrypt(
    salt: &Salt,
    nonce: &Nonce,
    ciphertext: &Ciphertext,
    passphrase: &str,
) -> Result<Vec<u8>, Error> {
    let passphrase = match passphrase {
        "" => crate::inquire::passphrase(),
        _ => passphrase.to_string(),
    };
    let key = argon2_key_derivation(passphrase.as_bytes(), salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&key).unwrap();
    match cipher.decrypt(nonce.into(), ciphertext.as_slice()) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => Err(Error::InvalidPassphrase),
    }
}
pub fn save(filename: &str, key: &Key) -> Result<(), Error> {
    let (salt, nonce, ciphertext) = encrypt(key)?;
    let mut bytes = [0; 92];
    bytes[0..32].copy_from_slice(&salt);
    bytes[32..44].copy_from_slice(&nonce);
    bytes[44..92].copy_from_slice(&ciphertext);
    let mut path = util::default_path().join(filename);
    path.set_extension(EXTENSION);
    let mut file = File::create(path).unwrap();
    file.write_all(hex::encode(bytes).as_bytes()).unwrap();
    Ok(())
}
pub fn load() -> Result<(Salt, Nonce, Ciphertext, Key), Error> {
    fn read_exact(path: impl AsRef<Path>) -> Result<[u8; 92], Error> {
        let mut file = File::open(path).unwrap();
        let mut bytes = [0; 184];
        file.read_exact(&mut bytes).unwrap();
        let vec = hex::decode(bytes).unwrap();
        Ok(vec.try_into().unwrap())
    }
    fn attempt(slice: &[u8], passphrase: &str) -> Result<(Salt, Nonce, Ciphertext, Key), Error> {
        fn inner(slice: &[u8], passphrase: &str) -> Result<(Salt, Nonce, Ciphertext, Key), Error> {
            let salt: Salt = slice[0..32].try_into().unwrap();
            let nonce: Nonce = slice[32..44].try_into().unwrap();
            let ciphertext: Ciphertext = slice[44..92].try_into().unwrap();
            let key = Key::from_slice(
                decrypt(&salt, &nonce, &ciphertext, passphrase)?
                    .as_slice()
                    .try_into()
                    .unwrap(),
            )
            .map_err(Error::Key)?;
            Ok((salt, nonce, ciphertext, key))
        }
        let res = inner(slice, passphrase);
        if let Err(Error::InvalidPassphrase) = res {
            println!("{}", INCORRECT.red())
        }
        res
    }
    let mut filename = crate::inquire::select().map_err(Error::Inquire)?;
    let res = if filename.as_str() == *GENERATE {
        Some(Key::generate())
    } else if filename.as_str() == *IMPORT {
        Some(inquire::import().map_err(Error::Inquire)?)
    } else {
        None
    };
    if let Some(key) = res {
        if !inquire::save() {
            return Ok(([0; 32], [0; 12], [0; 48], key));
        }
        filename = inquire::name().map_err(Error::Inquire)?;
        save(&filename, &key)?;
    }
    let mut path = util::default_path().join(filename);
    path.set_extension(EXTENSION);
    clear();
    println!("{}", path.to_string_lossy().green());
    let bytes = read_exact(path)?;
    loop {
        let passphrase = crate::inquire::passphrase();
        let res = attempt(&bytes, &passphrase);
        if let Err(Error::InvalidPassphrase) = res {
            continue;
        }
        return res;
    }
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
