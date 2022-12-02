use crate::Wallet;
use colored::*;
use crossterm::{event, terminal};
use inquire::{Confirm, CustomType, Select};
use pea_address as address;
use pea_api::{get, post};
use pea_core::constants::COIN;
use pea_stake::Stake;
use pea_time::Time;
use pea_transaction::Transaction;
use std::{ops::Range, process, time::Duration};
pub struct Options {
    pub api: String,
    pub time_api: bool,
}
pub struct Command {
    api: String,
    pub time_api: bool,
    wallet: Option<Wallet>,
    pub time: Time,
}
impl Command {
    pub fn new(options: Options) -> Command {
        Command {
            api: options.api,
            time_api: options.time_api,
            wallet: None,
            time: Time::new(),
        }
    }
    pub async fn sync_time(&mut self) {
        if self.time.sync().await {
            println!(
                "Successfully adjusted for time difference. System clock is {} of world clock.",
                format!(
                    "{:?} {}",
                    Duration::from_micros(self.time.diff.abs() as u64),
                    if self.time.diff.is_negative() { "behind" } else { "ahead" }
                )
                .to_string()
                .yellow()
            );
        } else {
            println!("{}", "Failed to adjust for time difference!".red());
        }
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
                Self::api(&self.api).await;
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
                self.transaction(self.wallet.as_ref().unwrap()).await;
                true
            }
            "Stake" => {
                self.stake(self.wallet.as_ref().unwrap()).await;
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
            "Subkeys" => self.subkeys().await,
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
    async fn subkeys(&mut self) -> bool {
        match Select::new(">>", vec!["Balance", "Withdraw", "Back"]).prompt().unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        }) {
            "Balance" => {
                self.subkeys_balance().await;
                true
            }
            "Withdraw" => {
                self.subkeys_withdraw().await;
                true
            }
            "Back" => false,
            _ => process::exit(0),
        }
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
                    println!("Account balance: {}", pea_int::to_string(balance).yellow());
                    if balance <= fee {
                        println!("{}", "Insufficient balance".red());
                        continue;
                    }
                    let amount = balance - fee;
                    println!("Withdrawing: {} = {} - {}", amount.to_string().yellow(), balance, fee);
                    let mut transaction = Transaction::new(key.public_key_bytes(), amount, fee, self.time.timestamp_secs());
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
    async fn api(api: &str) {
        match get::index(api).await {
            Ok(info) => println!("{}", info.green()),
            Err(err) => println!("{}", err.to_string().red()),
        };
        match get::sync(api).await {
            Ok(sync) => {
                println!("Synchronize {}", sync.sync.yellow());
                println!("Last block seen {}", sync.last.yellow());
                println!("Height {}", sync.height.to_string().yellow());
                println!("Peers {}", sync.peers.to_string().yellow());
            }
            Err(err) => println!("{}", err.to_string().red()),
        };
    }
    async fn balance(api: &str, address: &str) {
        match get::balance(api, address).await {
            Ok(balance) => match get::balance_staked(api, address).await {
                Ok(balance_staked) => println!(
                    "Account balance: {}, locked: {}",
                    pea_int::to_string(balance).yellow(),
                    pea_int::to_string(balance_staked).yellow()
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
                Ok(f) => Ok(pea_int::floor((f * COIN as f64) as u128) as f64 / COIN as f64),
                Err(_) => Err(()),
            })
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            })
            * COIN as f64) as u128
    }
    fn inquire_fee() -> u128 {
        CustomType::<u128>::new("Fee:")
            .with_formatter(&|i| format!("{} {}", i, if i == 1 { "satoshi" } else { "satoshis" }))
            .with_error_message("Please type a valid number")
            .with_help_message("Type the fee to use in satoshis")
            .with_parser(&|x| match x.parse::<u128>() {
                Ok(u) => Ok(pea_int::floor(u)),
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
    async fn transaction(&self, wallet: &Wallet) {
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
        let mut transaction = Transaction::new(address::public::decode(&address).unwrap(), amount, fee, self.time.timestamp_secs());
        transaction.sign(&wallet.key);
        println!("Hash: {}", hex::encode(transaction.hash()).cyan());
        match post::transaction(&self.api, &transaction).await {
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
    async fn stake(&self, wallet: &Wallet) {
        let deposit = Self::inquire_deposit();
        let amount = Self::inquire_amount();
        let fee = Self::inquire_fee();
        let send = Self::inquire_send();
        if !send {
            return;
        }
        let mut stake = Stake::new(deposit, amount, fee, self.time.timestamp_secs());
        stake.sign(&wallet.key);
        println!("Hash: {}", hex::encode(stake.hash()).cyan());
        match post::stake(&self.api, &stake).await {
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
