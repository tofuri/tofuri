use crate::{db, types, util};
use ed25519::signature::Signer;
use ed25519_dalek::{Keypair, PublicKey, Signature};
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub input: types::PublicKey,
    pub output: types::PublicKey,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    #[serde(with = "BigArray")]
    pub signature: types::Signature,
}
impl Transaction {
    pub fn from(output: types::PublicKey, amount: u64, fee: u64, timestamp: u64) -> Transaction {
        Transaction {
            input: [0; 32],
            output,
            amount,
            fee,
            timestamp,
            signature: [0; 64],
        }
    }
    pub fn new(output: types::PublicKey, amount: u64, fee: u64) -> Transaction {
        Transaction::from(output, amount, fee, util::timestamp())
    }
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&TransactionHeader::from(self)).unwrap())
    }
    pub fn sign(&mut self, keypair: &Keypair) {
        self.input = keypair.public.to_bytes();
        self.signature = keypair.sign(&self.hash()).to_bytes();
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        let public_key: PublicKey = PublicKey::from_bytes(&self.input)?;
        let signature: Signature = Signature::from_bytes(&self.signature)?;
        Ok(public_key.verify_strict(&self.hash(), &signature)?)
    }
    pub fn is_valid(&self) -> bool {
        // check if output is a valid ed25519 public key
        // strictly verify transaction signature
        PublicKey::from_bytes(&self.output).is_ok()
            && self.verify().is_ok()
            && self.timestamp <= util::timestamp()
            && self.input != self.output
    }
    pub fn put(&self, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::cf_handle_transactions(db)?,
            self.hash(),
            bincode::serialize(self)?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Transaction, Box<dyn Error>> {
        Ok(bincode::deserialize(
            &db.get_cf(db::cf_handle_transactions(db)?, hash)?
                .ok_or("transaction not found")?,
        )?)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionHeader {
    pub input: types::PublicKey,
    pub output: types::PublicKey,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
}
impl TransactionHeader {
    pub fn from(transaction: &Transaction) -> TransactionHeader {
        TransactionHeader {
            input: transaction.input,
            output: transaction.output,
            amount: transaction.amount,
            fee: transaction.fee,
            timestamp: transaction.timestamp,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[bench]
    fn bench_hash(b: &mut Bencher) {
        let transaction = Transaction::new([0; 32], 0, 0);
        b.iter(|| transaction.hash());
    }
    #[bench]
    fn bench_bincode_serialize(b: &mut Bencher) {
        let keypair = util::keygen();
        let mut transaction = Transaction::new([0; 32], 0, 0);
        transaction.sign(&keypair);
        println!("{:?}", transaction);
        println!("{:?}", bincode::serialize(&transaction));
        println!("{:?}", bincode::serialize(&transaction).unwrap().len());
        b.iter(|| bincode::serialize(&transaction));
    }
    #[bench]
    fn bench_bincode_deserialize(b: &mut Bencher) {
        let keypair = util::keygen();
        let mut transaction = Transaction::new([0; 32], 0, 0);
        transaction.sign(&keypair);
        let bytes = bincode::serialize(&transaction).unwrap();
        b.iter(|| {
            let _: Transaction = bincode::deserialize(&bytes).unwrap();
        });
    }
}
