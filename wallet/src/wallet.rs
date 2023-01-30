use crate::inquire::address;
use crate::inquire::amount;
use crate::inquire::deposit;
use crate::inquire::fee;
use crate::inquire::search;
use crate::inquire::send;
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
use inquire::Confirm;
use inquire::Select;
use pea_address::address;
use pea_address::secret;
use pea_api::get;
use pea_api::post;
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
                self.search().await;
                true
            }
            "Height" => {
                self.height().await;
                true
            }
            "API" => {
                self.api().await;
                true
            }
            "Address" => {
                self.address();
                true
            }
            "Balance" => {
                self.balance().await;
                true
            }
            "Send" => {
                self.transaction().await;
                true
            }
            "Stake" => {
                self.stake().await;
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
    async fn api(&self) {
        match get::index(&self.api).await {
            Ok(info) => println!("{}", info.green()),
            Err(err) => println!("{}", err.to_string().red()),
        };
        match get::sync(&self.api).await {
            Ok(sync) => {
                println!("Synchronize {}", sync.status.yellow());
                println!("Height {}", sync.height.to_string().yellow());
                println!("Last block seen {}", sync.last_seen.yellow());
            }
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    async fn balance(&self) {
        let address = address::encode(&self.key.as_ref().unwrap().address_bytes());
        match get::balance(&self.api, &address).await {
            Ok(balance) => match get::staked(&self.api, &address).await {
                Ok(staked) => println!("Account balance: {}, staked: {}", balance.yellow(), staked.yellow()),
                Err(err) => println!("{}", err.to_string().red()),
            },
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    async fn height(&self) {
        match get::height(&self.api).await {
            Ok(height) => println!("Latest block height is {}.", height.to_string().yellow()),
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    async fn transaction(&self) {
        let address = address();
        let amount = amount();
        let fee = fee();
        if !match Confirm::new("Send?").prompt() {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                process::exit(0)
            }
        } {
            return;
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
        match post::transaction(&self.api, &transaction_a.b()).await {
            Ok(res) => println!("{}", if res == "success" { res.green() } else { res.red() }),
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    async fn stake(&self) {
        let deposit = deposit();
        let amount = amount();
        let fee = fee();
        let send = send();
        if !send {
            return;
        }
        let stake_a = StakeA::sign(deposit, amount, fee, pea_util::timestamp(), self.key.as_ref().unwrap()).unwrap();
        println!("Hash: {}", hex::encode(stake_a.hash).cyan());
        match post::stake(&self.api, &stake_a.b()).await {
            Ok(res) => println!("{}", if res == "success" { res.green() } else { res.red() }),
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    async fn search(&self) {
        let search = search();
        if address::decode(&search).is_ok() {
            match get::balance(&self.api, &search).await {
                Ok(balance) => match get::staked(&self.api, &search).await {
                    Ok(staked) => println!("Address found\nAccount balance: {}, staked: {}", balance.yellow(), staked.yellow()),
                    Err(err) => println!("{}", err.to_string().red()),
                },
                Err(err) => println!("{}", err.to_string().red()),
            };
            return;
        } else if search.len() == 64 {
            if let Ok(block) = get::block(&self.api, &search).await {
                println!("Block found\n{block:?}");
                return;
            };
            if let Ok(transaction) = get::transaction(&self.api, &search).await {
                println!("Transaction found\n{transaction:?}");
                return;
            };
            if let Ok(stake) = get::stake(&self.api, &search).await {
                println!("Stake found\n{stake:?}");
                return;
            };
        } else if search.parse::<usize>().is_ok() {
            if let Ok(hash) = get::hash(&self.api, &search.parse::<usize>().unwrap()).await {
                if let Ok(block) = get::block(&self.api, &hash).await {
                    println!("Block found{block:?}");
                    return;
                };
                return;
            };
        }
        println!("{}", "Nothing found".red());
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
    let filename = crate::inquire::wallet_select()?;
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
