use crate::http;
use colored::*;
use futures::FutureExt;
use log::{error, info};
use pea_address as address;
use pea_api::{
    get::{self, Block},
    post,
};
use pea_core::{constants::NANOS, types, util};
use pea_key::Key;
use pea_transaction::Transaction;
use pea_wallet::Wallet;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    time::{Duration, SystemTime},
};
use tokio::net::TcpListener;
const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Charge {
    secret_key_bytes: types::SecretKeyBytes,
    amount: u128,
    timestamp: u32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payment {
    pub public: String,
    pub amount: u128,
    pub timestamp: u32,
}
pub struct PaymentProcessor {
    pub wallet: Wallet,
    pub api: String,
    pub confirmations: usize,
    pub expires_after_secs: u32,
    charges: Vec<Charge>,
    chain: Vec<Block>,
    subkey: usize,
}
impl PaymentProcessor {
    pub fn new<'a>(wallet: Wallet, api: String, confirmations: usize, expires_after_secs: u32) -> Self {
        Self {
            wallet,
            api,
            confirmations,
            expires_after_secs,
            chain: vec![],
            charges: vec![],
            subkey: 0,
        }
    }
    pub fn get_charges(&self) -> Vec<Payment> {
        let mut payments = vec![];
        for charge in self.charges.iter() {
            let key = Key::from_secret_key_bytes(&charge.secret_key_bytes);
            let public = key.public();
            payments.push(Payment {
                public,
                amount: charge.amount,
                timestamp: charge.timestamp,
            })
        }
        payments
    }
    pub async fn send(&self, address: &str, amount: u128, fee: u128) -> Result<(), Box<dyn Error>> {
        let mut transaction = Transaction::new(address::public::decode(address).unwrap(), amount, fee);
        transaction.sign(&self.wallet.key);
        post::transaction(&self.api, &transaction).await?;
        Ok(())
    }
    pub fn withdraw() {}
    pub fn charge(&mut self, amount: u128) -> Payment {
        let key = self.wallet.key.subkey(self.subkey);
        self.subkey += 1;
        let public = key.public();
        let timestamp = util::timestamp();
        let charge = Charge {
            secret_key_bytes: key.secret_key_bytes(),
            amount,
            timestamp,
        };
        self.charges.push(charge);
        Payment { public, amount, timestamp }
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
            for charge in self.charges.iter() {
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
        let mut i = 0;
        while i < self.charges.len() {
            let charge = &self.charges[i];
            let public = Key::from_secret_key_bytes(&charge.secret_key_bytes).public();
            if {
                let amount = match map.get(&public) {
                    Some(a) => *a,
                    None => 0,
                };
                charge.amount < amount
            } {
                charges.push(self.charges.remove(i));
            } else {
                i += 1;
            }
        }
        let mut payments = vec![];
        for charge in charges {
            let key = Key::from_secret_key_bytes(&charge.secret_key_bytes);
            let public = key.public();
            payments.push(Payment {
                public,
                amount: charge.amount,
                timestamp: charge.timestamp,
            })
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
    pub async fn next() {
        let mut nanos = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let secs = nanos / NANOS;
        nanos -= secs * NANOS;
        nanos = NANOS - nanos;
        tokio::time::sleep(Duration::from_nanos(nanos as u64)).await
    }
    pub async fn listen(&mut self, listener: TcpListener) -> Result<(), Box<dyn Error>> {
        info!("{} {} http://{}", "Enabled".green(), "HTTP API".cyan(), listener.local_addr()?.to_string().green());
        loop {
            tokio::select! {
                Ok(stream) = http::next(&listener).fuse() => if let Err(err) = http::handler(stream, &self).await {
                    error!("{}", err);
                },
                _ = Self::next().fuse() => match self.check().await {
                    Ok(vec) => if !vec.is_empty() {
                        info!("{:?}", vec);
                    },
                    Err(err) => error!("{}", err)
                },
            }
        }
    }
}
