use crate::http;
use colored::*;
use futures::FutureExt;
use log::{error, info};
use pea_address as address;
use pea_api::get::{self};
use pea_core::{types, util};
use pea_key::Key;
use pea_pay_core::{Charge, ChargeStatus, Payment};
use pea_pay_db as db;
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
    pub key: Key,
    pub api: String,
    pub bind_api: String,
    pub confirmations: usize,
    pub expires: u32,
    pub tps: f64,
    charges: HashMap<types::AddressBytes, Charge>,
    chain: Vec<types::api::Block>,
    subkey: u128,
}
impl PaymentProcessor {
    pub fn new(options: Options) -> Self {
        let wallet = PaymentProcessor::wallet(options.tempkey, options.wallet, options.passphrase);
        info!("Address {}", pea_address::address::encode(&wallet.key.address_bytes()).green());
        let db = PaymentProcessor::db(options.tempdb);
        Self {
            db,
            key: wallet.key,
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
    pub fn get_charges(&self) -> Vec<Payment> {
        let mut payments = vec![];
        for charge in self.charges.values() {
            payments.push(charge.payment(&self.key));
        }
        payments
    }
    pub fn get_charge(&self, hash: &[u8]) -> Option<Payment> {
        self.charges.get(hash).map(|x| x.payment(&self.key))
    }
    pub fn withdraw() {}
    pub fn charge(&mut self, amount: u128) -> Payment {
        let charge = Charge {
            amount,
            timestamp: util::timestamp(),
            status: ChargeStatus::Pending,
            subkey: self.subkey,
        };
        let payment = charge.payment(&self.key);
        db::charge::put(&self.db, &self.key, &charge).unwrap();
        self.charges.insert(charge.address_bytes(&self.key), charge);
        self.subkey += 1;
        payment
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
                let address = address::address::encode(&charge.address_bytes(&self.key));
                if transaction.output_address == address {
                    let amount = match map.get(&address) {
                        Some(a) => *a,
                        None => 0,
                    };
                    map.insert(address, amount + pea_int::from_string(&transaction.amount).unwrap());
                }
            }
        }
        let mut charges = vec![];
        for charge in self.charges.values_mut() {
            let address = address::address::encode(&charge.address_bytes(&self.key));
            let res = {
                let amount = match map.get(&address) {
                    Some(a) => *a,
                    None => 0,
                };
                charge.amount < amount
            };
            if res {
                charge.status = ChargeStatus::Completed;
                db::charge::put(&self.db, &self.key, charge).unwrap();
                charges.push(charge);
            } else if matches!(charge.status, ChargeStatus::New | ChargeStatus::Pending) && charge.timestamp < util::timestamp() - self.expires {
                charge.status = ChargeStatus::Expired;
                db::charge::put(&self.db, &self.key, charge).unwrap();
            }
        }
        let mut payments = vec![];
        for charge in charges {
            payments.push(charge.payment(&self.key))
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
