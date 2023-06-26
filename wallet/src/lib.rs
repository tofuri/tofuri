pub mod inquire;
use crate::inquire::GENERATE;
use crate::inquire::IMPORT;
use ::inquire::Confirm;
use ::inquire::Select;
use address::public;
use address::secret;
use api::BlockHex;
use api::Root;
use api::StakeHex;
use api::TransactionHex;
use chrono::Utc;
use clap::Parser;
use colored::*;
use crossterm::event;
use crossterm::terminal;
use key::Key;
use rand::rngs::OsRng;
use reqwest::Client;
use reqwest::Url;
use std::fs::create_dir_all;
use std::fs::read_dir;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::process;
pub const EXTENSION: &str = "tofuri";
const INCORRECT: &str = "Incorrect passphrase";
pub type Salt = [u8; 32];
pub type Nonce = [u8; 12];
pub type Ciphertext = [u8; 48];
#[derive(Debug)]
pub enum Error {
    Key(key::Error),
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Inquire(inquire::Error),
    InvalidPassphrase,
}
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, env = "API", default_value = "http://localhost:2021/")]
    pub api: Url,
}
#[derive(Debug, Clone)]
pub struct Wallet {
    key: Option<Key>,
    api: Url,
    client: Client,
}
impl Wallet {
    pub fn new(api: Url) -> Wallet {
        Wallet {
            key: None,
            api,
            client: Client::default(),
        }
    }
    pub async fn select(&mut self) -> bool {
        let mut vec = vec!["Wallet", "Search", "Height", "API", "Exit"];
        if self.key.is_some() {
            let mut v = vec!["Address", "Balance", "Send", "Stake", "Secret"];
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
            _ => {
                process::exit(0);
            }
        }
    }
    fn decrypt(&mut self) {
        let key = load().unwrap();
        self.key = Some(key);
    }
    async fn api(&self) -> Result<(), Error> {
        let root: Root = self
            .client
            .get(self.api.to_string())
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
        let address = public::encode(&self.key.as_ref().unwrap().address_bytes());
        let balance: String = self
            .client
            .get(format!("{}balance/{}", self.api.to_string(), address))
            .send()
            .await
            .map_err(Error::Reqwest)?
            .json()
            .await
            .map_err(Error::Reqwest)?;
        let staked: String = self
            .client
            .get(format!("{}staked/{}", self.api.to_string(), address))
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
            .get(format!("{}height", self.api.to_string()))
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
        let transaction = transaction::Transaction::sign(
            public::decode(&address).unwrap(),
            amount,
            fee,
            Utc::now().timestamp() as u32,
            self.key.as_ref().unwrap(),
        )
        .unwrap();
        println!("[u8; 32]: {}", hex::encode(transaction.hash()).cyan());
        let transaction_hex: TransactionHex = transaction.try_into().unwrap();
        let res: String = self
            .client
            .post(format!("{}transaction", self.api.to_string()))
            .json(&transaction_hex)
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
        let stake = stake::Stake::sign(
            deposit,
            amount,
            fee,
            Utc::now().timestamp() as u32,
            self.key.as_ref().unwrap(),
        )
        .unwrap();
        println!("[u8; 32]: {}", hex::encode(stake.hash()).cyan());
        let stake_hex: StakeHex = stake.try_into().unwrap();
        let res: String = self
            .client
            .post(format!("{}stake", self.api.to_string()))
            .json(&stake_hex)
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
        if public::decode(&search).is_ok() {
            let balance: String = self
                .client
                .get(format!("{}balance/{}", self.api.to_string(), search))
                .send()
                .await
                .map_err(Error::Reqwest)?
                .json()
                .await
                .map_err(Error::Reqwest)?;
            let staked: String = self
                .client
                .get(format!("{}staked/{}", self.api.to_string(), search))
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
                .get(format!("{}block/{}", self.api.to_string(), search))
                .send()
                .await
            {
                let block: BlockHex = res.json().await.map_err(Error::Reqwest)?;
                println!("Block found\n{block:?}");
                return Ok(());
            }
            if let Ok(res) = self
                .client
                .get(format!("{}/transaction/{}", self.api.to_string(), search))
                .send()
                .await
            {
                let transaction: TransactionHex = res.json().await.map_err(Error::Reqwest)?;
                println!("Transaction found\n{transaction:?}");
                return Ok(());
            }
            if let Ok(res) = self
                .client
                .get(format!("{}stake/{}", self.api.to_string(), search))
                .send()
                .await
            {
                let stake: StakeHex = res.json().await.map_err(Error::Reqwest)?;
                println!("Stake found\n{stake:?}");
                return Ok(());
            }
        } else if search.parse::<usize>().is_ok() {
            if let Ok(res) = self
                .client
                .get(format!("{}hash/{}", self.api.to_string(), search))
                .send()
                .await
            {
                let hash: String = res.json().await.map_err(Error::Reqwest)?;
                if let Ok(res) = self
                    .client
                    .get(format!("{}block/{}", self.api.to_string(), hash))
                    .send()
                    .await
                {
                    let block: BlockHex = res.json().await.map_err(Error::Reqwest)?;
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
            public::encode(&self.key.as_ref().unwrap().address_bytes()).green()
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
}
pub fn save(filename: &str, key: &Key) {
    let rng = &mut OsRng;
    let pwd = crate::inquire::new_passphrase();
    let encrypted = encryption::encrypt(rng, key.secret_key_bytes(), pwd);
    let mut path = default_path().join(filename);
    path.set_extension(EXTENSION);
    let mut file = File::create(path).unwrap();
    file.write_all(hex::encode(encrypted).as_bytes()).unwrap();
}
pub fn load() -> Result<Key, Error> {
    fn read_exact(path: impl AsRef<Path>) -> Result<[u8; 92], Error> {
        let mut file = File::open(path).unwrap();
        let mut bytes = [0; 184];
        file.read_exact(&mut bytes).unwrap();
        let vec = hex::decode(bytes).unwrap();
        Ok(vec.try_into().unwrap())
    }
    fn attempt(encrypted: &[u8; 92], passphrase: &str) -> Result<Key, Error> {
        fn inner(encrypted: &[u8; 92], passphrase: &str) -> Result<Key, Error> {
            let passphrase = match passphrase {
                "" => crate::inquire::passphrase(),
                _ => passphrase.to_string(),
            };
            let secret_key_bytes = match encryption::decrypt(encrypted, &passphrase) {
                Some(bytes) => bytes,
                None => return Err(Error::InvalidPassphrase),
            };
            let key = Key::from_slice(&secret_key_bytes).map_err(Error::Key)?;
            Ok(key)
        }
        let res = inner(encrypted, passphrase);
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
            return Ok(key);
        }
        filename = inquire::name().map_err(Error::Inquire)?;
        save(&filename, &key);
    }
    let mut path = default_path().join(filename);
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
pub fn default_path() -> &'static Path {
    Path::new("./tofuri-wallet")
}
pub fn filenames() -> Result<Vec<String>, io::Error> {
    let path = default_path();
    if !path.exists() {
        create_dir_all(path).unwrap();
    }
    let mut filenames: Vec<String> = vec![];
    for entry in read_dir(path).unwrap() {
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
