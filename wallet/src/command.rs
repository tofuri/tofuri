use crate::Wallet;
use colored::*;
use crossterm::{event, terminal};
use inquire::{Confirm, CustomType, Select};
use pea_address as address;
use pea_amount as amount;
use pea_api::{get, post};
use pea_core::constants::DECIMAL_PRECISION;
use pea_stake::Stake;
use pea_transaction::Transaction;
use std::{ops::Range, process, time::Duration};
pub struct Command {
    api: String,
    wallet: Option<Wallet>,
}
impl Command {
    pub fn new(api: String) -> Command {
        Command { api, wallet: None }
    }
    pub async fn select(&mut self) -> bool {
        let mut vec = vec!["Wallet", "Search", "Height", "API", "Exit"];
        if self.wallet.is_some() {
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
                Self::search(&self.api).await;
                true
            }
            "Height" => {
                Self::height(&self.api).await;
                true
            }
            "API" => {
                Self::info(&self.api).await;
                true
            }
            "Address" => {
                Self::address(self.wallet.as_ref().unwrap());
                true
            }
            "Balance" => {
                Self::balance(&self.api, &self.wallet.as_ref().unwrap().key.public()).await;
                true
            }
            "Send" => {
                Self::transaction(self.wallet.as_ref().unwrap(), &self.api).await;
                true
            }
            "Stake" => {
                Self::stake(self.wallet.as_ref().unwrap(), &self.api).await;
                true
            }
            "Secret" => {
                Self::key(self.wallet.as_ref().unwrap());
                true
            }
            "Encrypted" => {
                Self::data(self.wallet.as_ref().unwrap());
                true
            }
            "Subkeys" => {
                self.subkeys().await;
                true
            }
            _ => {
                process::exit(0);
            }
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
    fn decrypt(&mut self) {
        self.wallet = Some(Wallet::import("", "").unwrap());
    }
    async fn subkeys(&mut self) {
        match Select::new(">>", vec!["Balance", "Withdraw", "Back"]).prompt().unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        }) {
            "Balance" => self.subkeys_balance().await,
            "Withdraw" => self.subkeys_withdraw().await,
            "Back" => {}
            _ => process::exit(0),
        };
    }
    fn inquire_subkeys_range() -> Range<usize> {
        let start = CustomType::<usize>::new("Range Start:")
            .with_error_message("Please type a valid number")
            .with_help_message("The start of the range of subkeys to check")
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            });
        let end = CustomType::<usize>::new("Range End:")
            .with_error_message("Please type a valid number")
            .with_help_message("The end of the range of subkeys to check")
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            });
        start..end
    }
    async fn subkeys_balance(&self) {
        let key = &self.wallet.as_ref().unwrap().key;
        for n in Self::inquire_subkeys_range() {
            let subkey = key.subkey(n);
            let address = subkey.public();
            println!("{} {}", n.to_string().red(), address.green());
            Self::balance(&self.api, &address).await;
        }
    }
    async fn subkeys_withdraw(&self) {
        let fee = Self::inquire_fee();
        let key = &self.wallet.as_ref().unwrap().key;
        for n in Self::inquire_subkeys_range() {
            let subkey = key.subkey(n);
            let address = subkey.public();
            println!("{} {}", n.to_string().red(), address.green());
            match get::balance(&self.api, &address).await {
                Ok(balance) => {
                    if balance == 0 {
                        continue;
                    }
                    println!("Balance: {}", balance.to_string().yellow());
                    if balance <= fee {
                        println!("{}", "Insufficient balance".red());
                        continue;
                    }
                    let amount = balance - fee;
                    println!("Withdrawing: {} = {} - {}", amount.to_string().yellow(), balance, fee);
                    let mut transaction = Transaction::new(key.public_key_bytes(), amount, fee);
                    transaction.sign(&subkey);
                    println!("{:?}", transaction);
                    match post::transaction(&self.api, &transaction).await {
                        Ok(res) => println!("{}", if res == "success" { res.green() } else { res.red() }),
                        Err(err) => println!("{}", err.to_string().red()),
                    };
                }
                Err(err) => println!("{}", err.to_string().red()),
            };
        }
    }
    async fn info(api: &str) {
        match get::index(api).await {
            Ok(info) => println!("{}", info.green()),
            Err(err) => println!("{}", err.to_string().red()),
        };
        match get::info(api).await {
            Ok(info) => {
                if info.syncing {
                    println!("{}", "Downloading blockchain!".yellow());
                } else {
                    println!("Blockchain synchronized.");
                }
                println!("Latest block height is {}", info.height.to_string().yellow());
                println!("Tree size {}", info.tree_size.to_string().yellow());
                println!("Gossipsub peers {}", info.gossipsub_peers.to_string().yellow());
                println!("Heartbeats {}", info.heartbeats.to_string().yellow());
                println!("Lag {}", format!("{:?}", Duration::from_micros((info.lag * 1000_f64) as u64)).yellow());
            }
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    async fn balance(api: &str, address: &str) {
        match get::balance(api, address).await {
            Ok(balance) => match get::balance_staked(api, address).await {
                Ok(balance_staked) => println!(
                    "Account balance: {} | {}, locked: {} | {}.",
                    (balance as f64 / DECIMAL_PRECISION as f64).to_string().yellow(), // rounding error
                    balance,
                    (balance_staked as f64 / DECIMAL_PRECISION as f64).to_string().yellow(), // rounding error
                    balance_staked
                ),
                Err(err) => println!("{}", err.to_string().red()),
            },
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    async fn height(api: &str) {
        match get::height(api).await {
            Ok(height) => println!("Latest block height is {}.", height.to_string().yellow()),
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    fn inquire_address() -> String {
        CustomType::<String>::new("Address:")
            .with_error_message("Please enter a valid address")
            .with_help_message("Type the hex encoded address with 0x as prefix")
            .with_parser(&|x| match address::public::decode(x) {
                Ok(y) => Ok(address::public::encode(&y)),
                Err(_) => Err(()),
            })
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            })
    }
    fn inquire_amount() -> u128 {
        (CustomType::<f64>::new("Amount:")
            .with_formatter(&|i| format!("{:.18} pea", i))
            .with_error_message("Please type a valid number")
            .with_help_message("Type the amount to send using a decimal point as a separator")
            .with_parser(&|x| match x.parse::<f64>() {
                Ok(f) => Ok(amount::floor(&((f * DECIMAL_PRECISION as f64) as u128)) as f64 / DECIMAL_PRECISION as f64),
                Err(_) => Err(()),
            })
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            })
            * DECIMAL_PRECISION as f64) as u128
    }
    fn inquire_fee() -> u128 {
        CustomType::<u128>::new("Fee:")
            .with_formatter(&|i| format!("{} {}", i, if i == 1 { "satoshi" } else { "satoshis" }))
            .with_error_message("Please type a valid number")
            .with_help_message("Type the fee to use in satoshis")
            .with_parser(&|x| match x.parse::<u128>() {
                Ok(u) => Ok(amount::floor(&u)),
                Err(_) => Err(()),
            })
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            })
    }
    fn inquire_deposit() -> bool {
        match Select::new(">>", vec!["deposit", "withdraw"]).prompt().unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        }) {
            "deposit" => true,
            "withdraw" => false,
            _ => false,
        }
    }
    async fn transaction(wallet: &Wallet, api: &str) {
        let address = Self::inquire_address();
        let amount = Self::inquire_amount();
        let fee = Self::inquire_fee();
        if !match Confirm::new("Send?").prompt() {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                process::exit(0)
            }
        } {
            return;
        }
        let mut transaction = Transaction::new(address::public::decode(&address).unwrap(), amount, fee);
        transaction.sign(&wallet.key);
        println!("Hash: {}", hex::encode(transaction.hash()).cyan());
        match post::transaction(api, &transaction).await {
            Ok(res) => println!("{}", if res == "success" { res.green() } else { res.red() }),
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    fn inquire_send() -> bool {
        match Confirm::new("Send?").prompt() {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                process::exit(0)
            }
        }
    }
    async fn stake(wallet: &Wallet, api: &str) {
        let deposit = Self::inquire_deposit();
        let amount = Self::inquire_amount();
        let fee = Self::inquire_fee();
        let send = Self::inquire_send();
        if !send {
            return;
        }
        let mut stake = Stake::new(deposit, amount, fee);
        stake.sign(&wallet.key);
        println!("Hash: {}", hex::encode(stake.hash()).cyan());
        match post::stake(api, &stake).await {
            Ok(res) => println!("{}", if res == "success" { res.green() } else { res.red() }),
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    fn address(wallet: &Wallet) {
        println!("{}", wallet.key.public().green());
    }
    fn inquire_search() -> String {
        CustomType::<String>::new("Search:")
            .with_error_message("Please enter a valid Address, Hash or Number.")
            .with_help_message("Search Blockchain, Transactions, Addresses, Blocks and Stakes")
            .with_parser(&|x| {
                if address::public::decode(x).is_ok() || x.len() == 64 || x.parse::<usize>().is_ok() {
                    return Ok(x.to_string());
                }
                Err(())
            })
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            })
    }
    async fn search(api: &str) {
        let search = Self::inquire_search();
        if address::public::decode(&search).is_ok() {
            Self::balance(api, &search).await;
            return;
        } else if search.len() == 64 {
            if let Ok(block) = get::block(api, &search).await {
                println!("{:?}", block);
                return;
            };
            if let Ok(transaction) = get::transaction(api, &search).await {
                println!("{:?}", transaction);
                return;
            };
            if let Ok(stake) = get::stake(api, &search).await {
                println!("{:?}", stake);
                return;
            };
        } else if search.parse::<usize>().is_ok() {
            if let Ok(hash) = get::hash(api, &search.parse::<usize>().unwrap()).await {
                if let Ok(block) = get::block(api, &hash).await {
                    println!("{:?}", block);
                    return;
                };
                return;
            };
        }
        println!("{}", "Nothing found".red());
    }
    fn key(wallet: &Wallet) {
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
            println!("{}", wallet.key.secret().red());
        }
    }
    fn data(wallet: &Wallet) {
        println!(
            "{}{}{}",
            hex::encode(&wallet.salt).red(),
            hex::encode(&wallet.nonce).red(),
            hex::encode(&wallet.ciphertext).red()
        );
    }
}
