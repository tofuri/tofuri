use pea_core::*;
use pea_key::Key;
use serde::Deserialize;
use serde::Serialize;
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
    pub amount: u128,
    pub timestamp: u32,
    pub status: ChargeStatus,
    pub subkey_n: u128,
}
impl Charge {
    pub fn address_bytes(&self, key: &Key) -> AddressBytes {
        key.subkey(self.subkey_n).unwrap().address_bytes()
    }
    pub fn payment(&self, key: &Key) -> Payment {
        let address = pea_address::address::encode(&self.address_bytes(key));
        let status = status(&self.status);
        Payment {
            address,
            amount: self.amount,
            timestamp: self.timestamp,
            status,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payment {
    pub address: String,
    pub amount: u128,
    pub timestamp: u32,
    pub status: String,
}
