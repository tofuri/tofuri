use pea_api::get::{self, Block};
use pea_core::{
    types::{self, SecretKey},
    util,
};
#[derive(Clone)]
pub struct Payment {
    pub public_key: types::PublicKeyBytes,
    pub amount: types::Amount,
    pub created: types::Timestamp,
}
pub struct PaymentProcessor {
    pub secret_key: types::SecretKeyBytes,
    pub counter: usize,
    pub payments: Vec<Payment>,
    pub blocks: Vec<Block>,
    pub confirmations: usize,
}
impl PaymentProcessor {
    pub fn new(secret_key: types::SecretKeyBytes, confirmations: usize) -> Self {
        Self {
            secret_key,
            counter: 0,
            payments: vec![],
            blocks: vec![],
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
    pub async fn check(&self) {}
}
