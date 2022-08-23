use super::{
    constants::{MAX_STAKE, MIN_STAKE},
    db, util,
};
use ed25519::signature::Signer;
use ed25519_dalek::{Keypair, PublicKey, Signature};
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stake {
    pub public_key: [u8; 32],
    pub amount: u64,
    pub deposit: bool, // false -> widthdraw
    pub fee: u64,
    pub timestamp: u64,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}
impl Stake {
    pub fn from(deposit: bool, amount: u64, fee: u64, timestamp: u64) -> Stake {
        Stake {
            public_key: [0; 32],
            amount,
            deposit,
            fee,
            timestamp,
            signature: [0; 64],
        }
    }
    pub fn new(deposit: bool, amount: u64, fee: u64) -> Stake {
        Stake::from(deposit, amount, fee, util::timestamp())
    }
    pub fn hash(&self) -> [u8; 32] {
        util::hash(&bincode::serialize(&StakeHeader::from(self)).unwrap()).into()
    }
    pub fn sign(&mut self, keypair: &Keypair) {
        self.public_key = keypair.public.to_bytes();
        self.signature = keypair.sign(&self.hash()).to_bytes();
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        let public_key: PublicKey = PublicKey::from_bytes(&self.public_key)?;
        let signature: Signature = Signature::from_bytes(&self.signature)?;
        Ok(public_key.verify_strict(&self.hash(), &signature)?)
    }
    pub fn is_valid(&self) -> bool {
        self.verify().is_ok()
            && self.timestamp <= util::timestamp()
            && self.amount >= MIN_STAKE
            && self.amount <= MAX_STAKE
    }
    pub fn put(&self, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::cf_handle_stakes(db)?,
            self.hash(),
            bincode::serialize(self)?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Stake, Box<dyn Error>> {
        Ok(bincode::deserialize(
            &db.get_cf(db::cf_handle_stakes(db)?, hash)?
                .ok_or("stake not found")?,
        )?)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct StakeHeader {
    pub public_key: [u8; 32],
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
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
        println!("{:?}", stake);
        println!("{:?}", bincode::serialize(&stake));
        println!("{:?}", bincode::serialize(&stake).unwrap().len());
        b.iter(|| bincode::serialize(&stake));
    }
    #[bench]
    fn bench_bincode_deserialize(b: &mut Bencher) {
        let keypair = util::keygen();
        let mut stake = Stake::new(true, 0, 0);
        stake.sign(&keypair);
        let bytes = bincode::serialize(&stake).unwrap();
        b.iter(|| {
            let _: Stake = bincode::deserialize(&bytes).unwrap();
        });
    }
}
