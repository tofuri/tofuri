use pea_core::{types, util};
use pea_key::Key;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChargeStatus {
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Charge {
    pub secret_key_bytes: types::SecretKeyBytes,
    pub amount: u128,
    pub timestamp: u32,
    pub status: ChargeStatus,
    pub subkey: usize,
}
impl Charge {
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&self).unwrap())
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payment {
    pub public: String,
    pub amount: u128,
    pub timestamp: u32,
    pub status: String,
}
impl Payment {
    pub fn from(charge: &Charge) -> Payment {
        let key = Key::from_secret_key_bytes(&charge.secret_key_bytes);
        let public = key.public();
        let status = status(&charge.status);
        Payment {
            public,
            amount: charge.amount,
            timestamp: charge.timestamp,
            status,
        }
    }
}
