use colored::*;
use log::info;
use pea_address as address;
use pea_api_core::Block;
use pea_api_core::Transaction;
use pea_core::*;
use pea_key::Key;
use pea_pay_core::Charge;
use pea_pay_core::ChargeStatus;
use pea_pay_core::Payment;
use pea_pay_db as db;
use rocksdb::DBWithThreadMode;
use rocksdb::IteratorMode;
use rocksdb::SingleThreaded;
use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;
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
pub struct Pay {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub key: Key,
    pub api: String,
    pub bind_api: String,
    pub confirmations: usize,
    pub expires: u32,
    pub tps: f64,
    charges: HashMap<AddressBytes, Charge>,
    chain: Vec<Block>,
    subkey: u128,
}
impl Pay {
    pub fn new(key: Key, db: DBWithThreadMode<SingleThreaded>, options: Options) -> Self {
        Self {
            db,
            key,
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
            timestamp: pea_util::timestamp(),
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
        // self.update_chain().await?;
        println!("a");
        let mut transactions = vec![];
        for (i, block) in self.chain.iter().rev().enumerate() {
            if i + 1 < self.confirmations {
                continue;
            }
            for hash in block.transactions.iter() {
                let transaction: Transaction = reqwest::get(format!("{}/transaction/{}", &self.api, &hash)).await?.json().await?;
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
                    map.insert(address, amount + pea_int::from_str(&transaction.amount).unwrap());
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
            } else if matches!(charge.status, ChargeStatus::New | ChargeStatus::Pending) && charge.timestamp < pea_util::timestamp() - self.expires {
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
        let latest_block: Block = reqwest::get(format!("{}/block", &self.api)).await?.json().await?;
        println!("b");
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
            Some(block) => block.timestamp < pea_util::timestamp() - self.expires,
            None => false,
        } {
            self.chain.remove(0);
        }
        Ok(())
    }
    async fn reload_chain(&mut self) -> Result<(), Box<dyn Error>> {
        self.chain = vec![];
        let latest_block: Block = reqwest::get(format!("{}/block", &self.api)).await?.json().await?;
        let mut previous_hash = latest_block.hash;
        loop {
            let block: Block = reqwest::get(format!("{}/block/{}", &self.api, &previous_hash)).await?.json().await?;
            if block.previous_hash == "0000000000000000000000000000000000000000000000000000000000000000"
                || block.timestamp < pea_util::timestamp() - self.expires
            {
                break;
            }
            previous_hash = block.previous_hash.clone();
            self.chain.insert(0, block);
        }
        Ok(())
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
