use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use sha2::Digest;
use sha2::Sha256;
use tofuri_key::Error;
use tofuri_key::Key;
use vint::vint;
use vint::Vint;
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Stake {
    pub amount: Vint<4>,
    pub fee: Vint<4>,
    pub deposit: bool,
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}
impl Stake {
    pub fn sign(
        deposit: bool,
        amount: u128,
        fee: u128,
        timestamp: u32,
        key: &Key,
    ) -> Result<Stake, Error> {
        let mut stake = Stake {
            amount: vint!(amount),
            fee: vint!(fee),
            deposit,
            timestamp,
            signature: [0; 64],
        };
        stake.signature = key.sign(&stake.hash())?;
        Ok(stake)
    }
    pub fn hash(&self) -> [u8; 32] {
        let mut array = [0; 9];
        array[0..4].copy_from_slice(&self.timestamp.to_be_bytes());
        array[4..8].copy_from_slice(&self.fee.0);
        array[8] = if self.deposit { 1 } else { 0 };
        let mut hasher = Sha256::new();
        hasher.update(array);
        hasher.finalize().into()
    }
    pub fn input_address(&self) -> Result<[u8; 20], Error> {
        Ok(Key::address(&self.input_public_key()?))
    }
    pub fn input_public_key(&self) -> Result<[u8; 33], Error> {
        Key::recover(&self.hash(), &self.signature)
    }
}
impl Default for Stake {
    fn default() -> Stake {
        Stake {
            amount: Vint([0; 4]),
            fee: Vint([0; 4]),
            deposit: false,
            timestamp: 0,
            signature: [0; 64],
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash() {
        assert_eq!(
            Stake::default().hash(),
            [
                62, 112, 119, 253, 47, 102, 214, 137, 224, 206, 230, 167, 207, 91, 55, 191, 45,
                202, 124, 151, 154, 243, 86, 208, 163, 28, 188, 92, 133, 96, 92, 125
            ]
        );
    }
    #[test]
    fn bincode_serialize() {
        assert_eq!(bincode::serialize(&Stake::default()).unwrap().len(), 77);
    }
}
