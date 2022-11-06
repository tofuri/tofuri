use pea_api::get::{self, Block};
use pea_core::{
    types::{self, SecretKey},
    util,
};
#[derive(Debug, Clone)]
pub struct Payment {
    pub public_key: types::PublicKeyBytes,
    pub amount: types::Amount,
    pub created: types::Timestamp,
}
#[derive(Debug)]
pub struct PaymentProcessor {
    pub api: String,
    pub secret_key: types::SecretKeyBytes,
    pub counter: usize,
    pub payments: Vec<Payment>,
    pub blocks: Vec<Block>,
    pub latest_hashes: Vec<String>,
    pub confirmations: usize,
}
impl PaymentProcessor {
    pub fn new<'a>(api: String, secret_key: types::SecretKeyBytes, confirmations: usize) -> Self {
        Self {
            api,
            secret_key,
            counter: 0,
            payments: vec![],
            blocks: vec![],
            latest_hashes: vec![],
            confirmations,
        }
    }
    pub fn pay(&mut self, amount: types::Amount) -> Payment {
        let mut secret_key = self.secret_key.to_vec();
        secret_key.append(&mut self.counter.to_le_bytes().to_vec());
        let hash = util::hash(&secret_key);
        let secret_key = SecretKey::from_bytes(&hash).unwrap();
        let public_key: types::PublicKey = (&secret_key).into();
        let payment = Payment {
            public_key: public_key.to_bytes(),
            amount,
            created: util::timestamp(),
        };
        self.payments.push(payment.clone());
        self.counter += 1;
        payment
    }
    pub async fn check(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let data = get::data(&self.api).await?;
        let latest_hashes = data.states.dynamic.latest_hashes;
        if self.latest_hashes == latest_hashes {
            return Ok(()); // nothing changed
        }
        self.latest_hashes = latest_hashes;
        for hash in self.latest_hashes.iter() {
            let block = get::block(&self.api, hash).await?;
        }
        Ok(())
    }
}
