use crate::{
    inquire::{address, amount, deposit, fee, search, send},
    util::{load, Ciphertext, Nonce, Salt},
};
use colored::*;
use inquire::{Confirm, Select};
use pea_address as address;
use pea_api::{get, post};
use pea_core::util;
use pea_key::Key;
use pea_stake::StakeA;
use pea_transaction::TransactionA;
use std::process;
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
            let mut v = vec!["Address", "Balance", "Send", "Stake", "Secret", "Encrypted", "Subkeys"];
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
            "Encrypted" => {
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
        let address = address::address::encode(&self.key.as_ref().unwrap().address_bytes());
        match get::balance(&self.api, &address).await {
            Ok(balance) => match get::balance_staked(&self.api, &address).await {
                Ok(balance_staked) => println!("Account balance: {}, locked: {}", balance.yellow(), balance_staked.yellow()),
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
            address::address::decode(&address).unwrap(),
            amount,
            fee,
            util::timestamp(),
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
        let fee = fee();
        let send = send();
        if !send {
            return;
        }
        let stake_a = StakeA::sign(deposit, fee, util::timestamp(), self.key.as_ref().unwrap()).unwrap();
        println!("Hash: {}", hex::encode(stake_a.hash).cyan());
        match post::stake(&self.api, &stake_a.b()).await {
            Ok(res) => println!("{}", if res == "success" { res.green() } else { res.red() }),
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    async fn search(&self) {
        let search = search();
        if address::address::decode(&search).is_ok() {
            match get::balance(&self.api, &search).await {
                Ok(balance) => match get::balance_staked(&self.api, &search).await {
                    Ok(balance_staked) => println!("Address found\nAccount balance: {}, locked: {}", balance.yellow(), balance_staked.yellow()),
                    Err(err) => println!("{}", err.to_string().red()),
                },
                Err(err) => println!("{}", err.to_string().red()),
            };
            return;
        } else if search.len() == 64 {
            if let Ok(block) = get::block(&self.api, &search).await {
                println!("Block found\n{:?}", block);
                return;
            };
            if let Ok(transaction) = get::transaction(&self.api, &search).await {
                println!("Transaction found\n{:?}", transaction);
                return;
            };
            if let Ok(stake) = get::stake(&self.api, &search).await {
                println!("Stake found\n{:?}", stake);
                return;
            };
        } else if search.parse::<usize>().is_ok() {
            if let Ok(hash) = get::hash(&self.api, &search.parse::<usize>().unwrap()).await {
                if let Ok(block) = get::block(&self.api, &hash).await {
                    println!("Block found{:?}", block);
                    return;
                };
                return;
            };
        }
        println!("{}", "Nothing found".red());
    }
    fn address(&self) {
        println!("{}", address::address::encode(&self.key.as_ref().unwrap().address_bytes()).green());
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
            println!("{}", address::secret::encode(&self.key.as_ref().unwrap().secret_key_bytes()).red());
        }
    }
    fn data(&self) {
        println!(
            "{}{}{}",
            hex::encode(&self.salt).red(),
            hex::encode(&self.nonce).red(),
            hex::encode(&self.ciphertext).red()
        );
    }
}
