use crate::http;
use colored::*;
use futures::FutureExt;
use log::{error, info};
use pea_address as address;
use pea_api::{
    get::{self},
    post,
};
use pea_core::{types, util};
use pea_key::Key;
use pea_pay_core::{Charge, ChargeStatus, Payment};
use pea_pay_db as db;
use pea_transaction::Transaction;
use pea_wallet::Wallet;
use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
use std::{
    collections::HashMap,
    error::Error,
    time::{Duration, Instant},
};
use tempdir::TempDir;
use tokio::net::TcpListener;
pub struct Options<'a> {
    pub tempdb: bool,
    pub tempkey: bool,
    pub confirmations: usize,
    pub expires: u32,
    pub tps: f64,
    pub wallet: &'a str,
    pub passphrase: &'a str,
    pub api: String,
    pub bind_api: String,
}
pub struct PaymentProcessor {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub wallet: Wallet,
    pub api: String,
    pub bind_api: String,
    pub confirmations: usize,
    pub expires: u32,
    pub tps: f64,
    // charges: HashMap<usize, Charge>,
    charges: HashMap<types::Hash, Charge>,
    chain: Vec<types::api::Block>,
    subkey: usize,
}
impl PaymentProcessor {
    pub fn new(options: Options) -> Self {
        let wallet = PaymentProcessor::wallet(options.tempkey, options.wallet, options.passphrase);
        info!("PubKey is {}", address::public::encode(&wallet.key.public_key_bytes()).green());
        let db = PaymentProcessor::db(options.tempdb);
        Self {
            db,
            wallet,
            api: options.api,
            bind_api: options.bind_api,
            confirmations: options.confirmations,
            expires: options.expires,
            tps: options.tps,
            chain: vec![],
            charges: HashMap::new(),
            subkey: 0,
        }
    }
    fn db(tempdb: bool) -> DBWithThreadMode<SingleThreaded> {
        let tempdir = TempDir::new("peacash-pay-db").unwrap();
        let path: &str = match tempdb {
            true => tempdir.path().to_str().unwrap(),
            false => "./peacash-pay-db",
        };
        db::open(path)
    }
    fn wallet(tempkey: bool, wallet: &str, passphrase: &str) -> Wallet {
        match tempkey {
            true => Wallet::new(),
            false => Wallet::import(wallet, passphrase).unwrap(),
        }
    }
    pub fn get_charges(&self) -> Vec<(String, Payment)> {
        let mut payments = vec![];
        for (hash, charge) in self.charges.iter() {
            payments.push((hex::encode(hash), Payment::from(charge)));
        }
        payments
    }
    pub fn get_charge(&self, hash: &[u8]) -> Option<Payment> {
        self.charges.get(hash).map(Payment::from)
    }
    pub async fn send(&self, address: &str, amount: u128, fee: u128) -> Result<(), Box<dyn Error>> {
        let mut transaction = Transaction::new(address::public::decode(address).unwrap(), amount, fee, util::timestamp());
        transaction.sign(&self.wallet.key);
        post::transaction(&self.api, &transaction).await?;
        Ok(())
    }
    pub fn withdraw() {}
    pub fn charge(&mut self, amount: u128) -> (String, Payment) {
        let key = self.wallet.key.subkey(self.subkey);
        let timestamp = util::timestamp();
        let charge = Charge {
            secret_key_bytes: key.secret_key_bytes(),
            amount,
            timestamp,
            status: ChargeStatus::Pending,
            subkey: self.subkey,
        };
        let payment = Payment::from(&charge);
        let hash = charge.hash();
        db::charge::put(&self.db, &charge).unwrap();
        self.charges.insert(hash, charge);
        self.subkey += 1;
        (hex::encode(hash), payment)
    }
    pub async fn check(&mut self) -> Result<Vec<Payment>, Box<dyn Error>> {
        self.update_chain().await?;
        let mut transactions = vec![];
        for (i, block) in self.chain.iter().rev().enumerate() {
            if i + 1 < self.confirmations {
                continue;
            }
            for hash in block.transactions.iter() {
                let transaction = get::transaction(&self.api, hash).await?;
                transactions.push(transaction);
            }
        }
        let mut map: HashMap<String, u128> = HashMap::new();
        for transaction in transactions {
            for charge in self.charges.values() {
                let public = Key::from_secret_key_bytes(&charge.secret_key_bytes).public();
                if transaction.public_key_output == public {
                    let amount = match map.get(&public) {
                        Some(a) => *a,
                        None => 0,
                    };
                    map.insert(public, amount + transaction.amount);
                }
            }
        }
        let mut charges = vec![];
        for charge in self.charges.values_mut() {
            let public = Key::from_secret_key_bytes(&charge.secret_key_bytes).public();
            let res = {
                let amount = match map.get(&public) {
                    Some(a) => *a,
                    None => 0,
                };
                charge.amount < amount
            };
            if res {
                charge.status = ChargeStatus::Completed;
                db::charge::put(&self.db, charge).unwrap();
                charges.push(charge);
            } else if matches!(charge.status, ChargeStatus::New | ChargeStatus::Pending) && charge.timestamp < util::timestamp() - self.expires {
                charge.status = ChargeStatus::Expired;
                db::charge::put(&self.db, charge).unwrap();
            }
        }
        let mut payments = vec![];
        for charge in charges {
            payments.push(Payment::from(charge))
        }
        Ok(payments)
    }
    async fn update_chain(&mut self) -> Result<(), Box<dyn Error>> {
        let latest_block = get::latest_block(&self.api).await?;
        if match self.chain.last() {
            Some(block) => block.hash == latest_block.hash,
            None => false,
        } {
            return Ok(());
        }
        if match self.chain.last() {
            Some(block) => block.hash == latest_block.previous_hash,
            None => false,
        } {
            self.chain.push(latest_block);
        } else {
            self.reload_chain().await?;
        }
        while match self.chain.first() {
            Some(block) => block.timestamp < util::timestamp() - self.expires,
            None => false,
        } {
            self.chain.remove(0);
        }
        Ok(())
    }
    async fn reload_chain(&mut self) -> Result<(), Box<dyn Error>> {
        self.chain = vec![];
        let latest_block = get::latest_block(&self.api).await?;
        let mut previous_hash = latest_block.hash;
        loop {
            let block = get::block(&self.api, &previous_hash).await?;
            if block.previous_hash == "0000000000000000000000000000000000000000000000000000000000000000" || block.timestamp < util::timestamp() - self.expires {
                break;
            }
            previous_hash = block.previous_hash.clone();
            self.chain.insert(0, block);
        }
        Ok(())
    }
    pub async fn start(&mut self) {
        self.load();
        let listener = TcpListener::bind(&self.bind_api).await.unwrap();
        info!(
            "API is listening on {}{}",
            "http://".cyan(),
            listener.local_addr().unwrap().to_string().magenta()
        );
        let mut interval = tokio::time::interval(Duration::from_micros(util::micros_per_tick(self.tps)));
        loop {
            tokio::select! {
                _ = interval.tick() => match self.check().await {
                    Ok(vec) => if !vec.is_empty() {
                        info!("{:?}", vec);
                    }
                    Err(err) => error!("{}", err)
                },
                res = listener.accept().fuse() => match res {
                    Ok((stream, socket_addr)) => {
                        match http::handler(stream, self).await {
                            Ok(first) => info!("{} {} {}", "API".cyan(), socket_addr.to_string().magenta(), first),
                            Err(err) => error!("{} {} {}", "API".cyan(), socket_addr.to_string().magenta(), err)
                        }
                    }
                    Err(err) => error!("{} {}", "API".cyan(), err)
                }
            }
        }
    }
    pub fn load(&mut self) {
        let start = Instant::now();
        for res in self.db.iterator_cf(db::charges(&self.db), IteratorMode::Start) {
            self.subkey += 1;
            let (hash, bytes) = res.unwrap();
            let hash = hash.to_vec().try_into().unwrap();
            let charge: Charge = bincode::deserialize(&bytes).unwrap();
            if matches!(charge.status, ChargeStatus::New | ChargeStatus::Pending) {
                self.charges.insert(hash, charge);
            }
        }
        info!("Loaded charges in {}", format!("{:?}", start.elapsed()).yellow());
    }
}
