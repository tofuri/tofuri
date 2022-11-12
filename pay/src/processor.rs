use crate::http;
use colored::*;
use futures::FutureExt;
use log::{error, info};
use pea_address as address;
use pea_api::{
    get::{self, Block},
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
    time::{Duration, Instant, SystemTime},
};
use tokio::net::TcpListener;
const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";
pub struct PaymentProcessor {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub wallet: Wallet,
    pub api: String,
    pub confirmations: usize,
    pub expires_after_secs: u32,
    // charges: HashMap<usize, Charge>,
    charges: HashMap<types::Hash, Charge>,
    chain: Vec<Block>,
    subkey: usize,
}
impl PaymentProcessor {
    pub fn new(db: DBWithThreadMode<SingleThreaded>, wallet: Wallet, api: String, confirmations: usize, expires_after_secs: u32) -> Self {
        Self {
            db,
            wallet,
            api,
            confirmations,
            expires_after_secs,
            chain: vec![],
            charges: HashMap::new(),
            subkey: 0,
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
        match self.charges.get(hash) {
            Some(charge) => Some(Payment::from(charge)),
            None => None,
        }
    }
    pub async fn send(&self, address: &str, amount: u128, fee: u128) -> Result<(), Box<dyn Error>> {
        let mut transaction = Transaction::new(address::public::decode(address).unwrap(), amount, fee);
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
        (hex::encode(&hash), payment)
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
            if {
                let amount = match map.get(&public) {
                    Some(a) => *a,
                    None => 0,
                };
                charge.amount < amount
            } {
                charge.status = ChargeStatus::Completed;
                db::charge::put(&self.db, &charge).unwrap();
                charges.push(charge);
            } else if matches!(charge.status, ChargeStatus::New | ChargeStatus::Pending) && charge.timestamp < util::timestamp() - self.expires_after_secs {
                charge.status = ChargeStatus::Expired;
                db::charge::put(&self.db, &charge).unwrap();
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
            Some(block) => block.timestamp < util::timestamp() - self.expires_after_secs,
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
            if block.previous_hash == GENESIS || block.timestamp < util::timestamp() - self.expires_after_secs {
                break;
            }
            previous_hash = block.previous_hash.clone();
            self.chain.insert(0, block);
        }
        Ok(())
    }
    async fn next(tps: f64) {
        let f = 1 as f64 / tps;
        let u = (f * 1_000_000_000 as f64) as u128;
        let mut nanos = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let secs = nanos / u;
        nanos -= secs * u;
        let nanos = (u - nanos) as u64;
        tokio::time::sleep(Duration::from_nanos(nanos)).await
    }
    pub async fn listen(&mut self, listener: TcpListener, tps: f64) -> Result<(), Box<dyn Error>> {
        info!("{} {} http://{}", "Enabled".green(), "HTTP API".cyan(), listener.local_addr()?.to_string().green());
        loop {
            tokio::select! {
                Ok(stream) = http::next(&listener).fuse() => if let Err(err) = http::handler(stream, self).await {
                    error!("{}", err);
                },
                _ = Self::next(tps).fuse() => match self.check().await {
                    Ok(vec) => if !vec.is_empty() {
                        info!("{:?}", vec);
                    },
                    Err(err) => error!("{}", err)
                },
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
        info!("{} {}", "Charges load".cyan(), format!("{:?}", start.elapsed()).yellow());
    }
}
