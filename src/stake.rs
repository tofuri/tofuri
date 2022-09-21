use crate::{amount, constants::MAX_STAKE, db, types, util};
use ed25519::signature::Signer;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stake {
    pub public_key: types::PublicKeyBytes,
    pub amount: types::Amount,
    pub deposit: bool, // false -> withdraw
    pub fee: types::Amount,
    pub timestamp: types::Timestamp,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Stake {
    pub fn new(deposit: bool, amount: types::Amount, fee: types::Amount) -> Stake {
        Stake {
            public_key: [0; 32],
            amount,
            deposit,
            fee,
            timestamp: util::timestamp(),
            signature: [0; 64],
        }
    }
    pub fn from(stake: &CompressedStake) -> Stake {
        Stake {
            public_key: stake.public_key,
            amount: amount::from_bytes(&stake.amount),
            fee: amount::from_bytes(&stake.fee),
            deposit: stake.deposit,
            timestamp: stake.timestamp,
            signature: stake.signature,
        }
    }
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&StakeHeader::from(self)).unwrap())
    }
    pub fn sign(&mut self, keypair: &types::Keypair) {
        self.public_key = keypair.public.to_bytes();
        self.signature = keypair.sign(&self.hash()).to_bytes();
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        let public_key = types::PublicKey::from_bytes(&self.public_key)?;
        let signature = types::Signature::from_bytes(&self.signature)?;
        Ok(public_key.verify_strict(&self.hash(), &signature)?)
    }
    pub fn put(&self, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::stakes(db),
            self.hash(),
            bincode::serialize(&CompressedStake::from(self))?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Stake, Box<dyn Error>> {
        let compressed: CompressedStake =
            bincode::deserialize(&db.get_cf(db::stakes(db), hash)?.ok_or("stake not found")?)?;
        Ok(Stake::from(&compressed))
    }
    pub fn validate(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        balance: types::Amount,
        balance_staked: types::Amount,
        timestamp: types::Timestamp,
    ) -> Result<(), Box<dyn Error>> {
        if !self.verify().is_ok() {
            return Err("stake has invalid signature".into());
        }
        if self.amount == 0 {
            return Err("stake has invalid amount".into());
        }
        if self.fee == 0 {
            return Err("stake invalid fee".into());
        }
        if self.timestamp > util::timestamp() {
            return Err("stake has invalid timestamp (stake is from the future)".into());
        }
        if Stake::get(db, &self.hash()).is_ok() {
            return Err("stake already in chain".into());
        }
        if self.deposit {
            if self.amount + self.fee > balance {
                return Err("stake deposit too expensive".into());
            }
            if self.amount + balance_staked > MAX_STAKE {
                return Err("stake deposit exceeds MAX_STAKE".into());
            }
        } else {
            if self.fee > balance {
                return Err("stake withdraw insufficient funds".into());
            }
            if self.amount > balance_staked {
                return Err("stake withdraw too expensive".into());
            }
        }
        if self.timestamp < timestamp {
            return Err("stake too old".into());
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct StakeHeader {
    pub public_key: types::PublicKeyBytes,
    pub amount: types::Amount,
    pub fee: types::Amount,
    pub timestamp: types::Timestamp,
}
impl StakeHeader {
    pub fn from(stake: &Stake) -> StakeHeader {
        StakeHeader {
            public_key: stake.public_key,
            amount: stake.amount,
            fee: stake.fee,
            timestamp: stake.timestamp,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompressedStake {
    pub public_key: types::PublicKeyBytes,
    pub amount: types::CompressedAmount,
    pub fee: types::CompressedAmount,
    pub deposit: bool,
    pub timestamp: types::Timestamp,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl CompressedStake {
    pub fn from(stake: &Stake) -> CompressedStake {
        CompressedStake {
            public_key: stake.public_key,
            amount: amount::to_bytes(&stake.amount),
            fee: amount::to_bytes(&stake.fee),
            deposit: stake.deposit,
            timestamp: stake.timestamp,
            signature: stake.signature,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[bench]
    fn bench_hash(b: &mut Bencher) {
        let stake = Stake::new(true, 0, 0);
        b.iter(|| stake.hash());
    }
    #[bench]
    fn bench_bincode_serialize(b: &mut Bencher) {
        let keypair = util::keygen();
        let mut stake = Stake::new(true, 0, 0);
        stake.sign(&keypair);
        let compressed = CompressedStake::from(&stake);
        println!("{:?}", compressed);
        println!("{:?}", bincode::serialize(&compressed));
        println!("{:?}", bincode::serialize(&compressed).unwrap().len());
        b.iter(|| bincode::serialize(&compressed));
    }
    #[bench]
    fn bench_bincode_deserialize(b: &mut Bencher) {
        let keypair = util::keygen();
        let mut stake = Stake::new(true, 0, 0);
        stake.sign(&keypair);
        let compressed = CompressedStake::from(&stake);
        let bytes = bincode::serialize(&compressed).unwrap();
        b.iter(|| {
            let _: CompressedStake = bincode::deserialize(&bytes).unwrap();
        });
    }
}
