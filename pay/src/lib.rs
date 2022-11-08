use pea_address as address;
use pea_api::{
    get::{self, Block},
    post,
};
use pea_core::{types, util};
use pea_key::Key;
use pea_transaction::Transaction;
use std::collections::HashMap;
const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";
#[derive(Debug, Clone)]
pub struct Payment {
    pub address: types::Address,
    pub amount: types::Amount,
    pub created: types::Timestamp,
}
#[derive(Debug)]
pub struct PaymentProcessor {
    pub api: String,
    pub key: Key,
    pub counter: usize,
    pub payments: Vec<Payment>,
    pub confirmations: usize,
    pub expires_after_secs: u32,
    pub chain: Vec<Block>,
    pub withdrawal_address: types::Address,
}
impl PaymentProcessor {
    pub fn new<'a>(api: String, key: Key, confirmations: usize, expires_after_secs: u32, withdrawal_address: types::Address) -> Self {
        Self {
            api,
            key,
            counter: 0,
            payments: vec![],
            confirmations,
            expires_after_secs,
            chain: vec![],
            withdrawal_address,
        }
    }
    pub async fn send(&self, address: &str, amount: types::Amount, fee: types::Amount) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = Transaction::new(address::public::decode(address).unwrap(), amount, fee);
        transaction.sign(&self.key);
        post::transaction(&self.api, &transaction).await?;
        Ok(())
    }
    pub fn withdraw() {}
    pub fn charge(&mut self, amount: types::Amount) -> Payment {
        let mut secret_key = self.key.secret_key_bytes().to_vec();
        secret_key.append(&mut self.counter.to_le_bytes().to_vec());
        let hash = util::hash(&secret_key);
        let key = Key::from_secret_key_bytes(&hash);
        let address = key.public();
        let created = util::timestamp();
        let payment = Payment { address, amount, created };
        self.payments.push(payment.clone());
        self.counter += 1;
        payment
    }
    pub async fn check(&mut self) -> Result<Vec<Payment>, Box<dyn std::error::Error>> {
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
        let mut map: HashMap<String, types::Amount> = HashMap::new();
        for transaction in transactions {
            for payment in self.payments.iter() {
                if transaction.public_key_output == payment.address {
                    let amount = match map.get(&payment.address) {
                        Some(a) => *a,
                        None => 0,
                    };
                    map.insert(payment.address.clone(), amount + transaction.amount);
                }
            }
        }
        let mut vec = vec![];
        let mut i = 0;
        while i < self.payments.len() {
            let payment = &self.payments[i];
            if {
                let amount = match map.get(&payment.address) {
                    Some(a) => *a,
                    None => 0,
                };
                payment.amount < amount
            } {
                vec.push(self.payments.remove(i));
            } else {
                i += 1;
            }
        }
        Ok(vec)
    }
    async fn update_chain(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    async fn reload_chain(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
}
