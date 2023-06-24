pub mod db;
pub mod router;
use api::BlockHex;
use api::TransactionHex;
use chrono::Utc;
use clap::Parser;
use db::charge;
use decimal::FromStr;
use key::Key;
use reqwest::Client;
use reqwest::Url;
use rocksdb::IteratorMode;
use rocksdb::DB;
use std::collections::HashMap;
use std::num::ParseIntError;
use tracing::instrument;
#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    RocksDB(rocksdb::Error),
    DB(db::Error),
    Bincode(bincode::Error),
    ParseIntError(ParseIntError),
    TryFromSliceError(core::array::TryFromSliceError),
}
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// Store blockchain in a temporary database
    #[clap(long, env = "TEMPDB")]
    pub tempdb: bool,

    /// Confirmations needed
    #[clap(long, env = "CONFIRMATIONS", default_value_t = 10)]
    pub confirmations: usize,

    /// Charge expires after seconds
    #[clap(long, env = "EXPIRES", default_value_t = 60)]
    pub expires: u32,

    /// API Endpoint
    #[clap(long, env = "API", default_value = "http://localhost:2021/")]
    pub api: Url,

    /// Pay API Endpoint
    #[clap(long, env = "PAY_API", default_value = "[::]:2023")]
    pub pay_api: String,

    /// Secret key
    #[clap(long, env = "SECRET")]
    pub secret: String,

    /// Disable tracing_subscriber timestamps
    #[clap(long, env = "WITHOUT_TIME")]
    pub without_time: bool,
}
#[derive(Debug)]
pub struct Pay {
    pub db: DB,
    pub key: Key,
    pub args: Args,
    charges: HashMap<[u8; 20], Charge>,
    chain: Vec<BlockHex>,
    subkey_n: u128,
    client: Client,
}
impl Pay {
    pub fn new(db: DB, key: Key, args: Args) -> Pay {
        Pay {
            db,
            key,
            args,
            chain: vec![],
            charges: HashMap::new(),
            subkey_n: 0,
            client: Client::new(),
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
    pub fn charge(&mut self, amount: u128) -> Result<Payment, Error> {
        let charge = Charge {
            amount,
            timestamp: Utc::now().timestamp() as u32,
            status: ChargeStatus::Pending,
            subkey_n: self.subkey_n,
        };
        let payment = charge.payment(&self.key);
        charge::put(&self.db, &self.key, &charge).map_err(Error::DB)?;
        self.charges.insert(charge.address_bytes(&self.key), charge);
        self.subkey_n += 1;
        Ok(payment)
    }
    pub async fn check(&mut self) -> Result<Vec<Payment>, Error> {
        self.update_chain().await?;
        let mut transactions = vec![];
        for (i, block) in self.chain.iter().rev().enumerate() {
            if i + 1 < self.args.confirmations {
                continue;
            }
            for hash in block.transactions.iter() {
                let transaction: TransactionHex = self
                    .client
                    .get(format!(
                        "{}transaction/{}",
                        &self.args.api.to_string(),
                        &hash
                    ))
                    .send()
                    .await
                    .map_err(Error::Reqwest)?
                    .json()
                    .await
                    .map_err(Error::Reqwest)?;
                transactions.push(transaction);
            }
        }
        let mut map: HashMap<String, u128> = HashMap::new();
        for transaction in transactions {
            for charge in self.charges.values() {
                let address = address::public::encode(&charge.address_bytes(&self.key));
                if transaction.output_address == address {
                    let amount = match map.get(&address) {
                        Some(a) => *a,
                        None => 0,
                    };
                    map.insert(
                        address,
                        amount
                            + u128::from_str::<18>(&transaction.amount)
                                .map_err(Error::ParseIntError)?,
                    );
                }
            }
        }
        let mut charges = vec![];
        for charge in self.charges.values_mut() {
            let address = address::public::encode(&charge.address_bytes(&self.key));
            let res = {
                let amount = match map.get(&address) {
                    Some(a) => *a,
                    None => 0,
                };
                charge.amount < amount
            };
            if res {
                charge.status = ChargeStatus::Completed;
                charge::put(&self.db, &self.key, charge).map_err(Error::DB)?;
                charges.push(charge);
            } else if matches!(charge.status, ChargeStatus::New | ChargeStatus::Pending)
                && charge.timestamp < Utc::now().timestamp() as u32 - self.args.expires
            {
                charge.status = ChargeStatus::Expired;
                charge::put(&self.db, &self.key, charge).map_err(Error::DB)?;
            }
        }
        let mut payments = vec![];
        for charge in charges {
            payments.push(charge.payment(&self.key))
        }
        Ok(payments)
    }
    async fn update_chain(&mut self) -> Result<(), Error> {
        let latest_block: BlockHex = self
            .client
            .get(format!("{}block", &self.args.api.to_string()))
            .send()
            .await
            .map_err(Error::Reqwest)?
            .json()
            .await
            .map_err(Error::Reqwest)?;
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
            Some(block) => block.timestamp < Utc::now().timestamp() as u32 - self.args.expires,
            None => false,
        } {
            self.chain.remove(0);
        }
        Ok(())
    }
    async fn reload_chain(&mut self) -> Result<(), Error> {
        let mut chain = vec![];
        let mut previous_hash = self
            .client
            .get(format!("{}block", &self.args.api.to_string()))
            .send()
            .await
            .map_err(Error::Reqwest)?
            .json::<BlockHex>()
            .await
            .map_err(Error::Reqwest)?
            .hash;
        loop {
            let block: BlockHex = self
                .client
                .get(format!(
                    "{}block/{}",
                    &self.args.api.to_string(),
                    &previous_hash
                ))
                .send()
                .await
                .map_err(Error::Reqwest)?
                .json()
                .await
                .map_err(Error::Reqwest)?;
            if let Some(latest_block) = self.chain.last() {
                if latest_block.hash == block.previous_hash {
                    self.chain.append(&mut chain);
                    return Ok(());
                }
            }
            if block.previous_hash
                == "0000000000000000000000000000000000000000000000000000000000000000"
                || block.timestamp < Utc::now().timestamp() as u32 - self.args.expires
            {
                break;
            }
            previous_hash = block.previous_hash.clone();
            chain.insert(0, block);
        }
        self.chain = chain;
        Ok(())
    }
    #[instrument(skip_all)]
    pub fn load(&mut self) -> Result<(), Error> {
        for res in self
            .db
            .iterator_cf(charge::cf(&self.db), IteratorMode::Start)
        {
            self.subkey_n += 1;
            let (hash, bytes) = res.map_err(Error::RocksDB)?;
            let hash = hash.to_vec().try_into().unwrap();
            let charge: Charge = bincode::deserialize(&bytes).map_err(Error::Bincode)?;
            if matches!(charge.status, ChargeStatus::New | ChargeStatus::Pending) {
                self.charges.insert(hash, charge);
            }
        }
        Ok(())
    }
}
use serde::Deserialize;
use serde::Serialize;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ChargeStatus {
    #[default]
    New,
    Pending,
    Expired,
    Completed,
    Cancelled,
}
pub fn status(status: &ChargeStatus) -> String {
    match *status {
        ChargeStatus::New => "NEW".to_string(),
        ChargeStatus::Pending => "PENDING".to_string(),
        ChargeStatus::Expired => "EXPIRED".to_string(),
        ChargeStatus::Completed => "COMPLETED".to_string(),
        ChargeStatus::Cancelled => "CANCELLED".to_string(),
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Charge {
    pub amount: u128,
    pub timestamp: u32,
    pub status: ChargeStatus,
    pub subkey_n: u128,
}
impl Charge {
    pub fn address_bytes(&self, key: &Key) -> [u8; 20] {
        key.subkey(self.subkey_n).unwrap().address_bytes()
    }
    pub fn payment(&self, key: &Key) -> Payment {
        let address = address::public::encode(&self.address_bytes(key));
        let status = status(&self.status);
        Payment {
            address,
            amount: self.amount,
            timestamp: self.timestamp,
            status,
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Payment {
    pub address: String,
    pub amount: u128,
    pub timestamp: u32,
    pub status: String,
}
