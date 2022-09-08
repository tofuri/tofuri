use crate::{amount, db, types, util};
use ed25519::signature::Signer;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: types::Amount,
    pub fee: types::Amount,
    pub timestamp: types::Timestamp,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Transaction {
    pub fn new(
        public_key_output: types::PublicKeyBytes,
        amount: types::Amount,
        fee: types::Amount,
    ) -> Transaction {
        Transaction {
            public_key_input: [0; 32],
            public_key_output,
            amount,
            fee,
            timestamp: util::timestamp(),
            signature: [0; 64],
        }
    }
    pub fn from(transaction: &CompressedTransaction) -> Transaction {
        Transaction {
            public_key_input: transaction.public_key_input,
            public_key_output: transaction.public_key_output,
            amount: amount::from_bytes(transaction.amount),
            fee: amount::from_bytes(transaction.fee),
            timestamp: transaction.timestamp,
            signature: transaction.signature,
        }
    }
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&TransactionHeader::from(self)).unwrap())
    }
    pub fn sign(&mut self, keypair: &types::Keypair) {
        self.public_key_input = keypair.public.to_bytes();
        self.signature = keypair.sign(&self.hash()).to_bytes();
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        let public_key = types::PublicKey::from_bytes(&self.public_key_input)?;
        let signature = types::Signature::from_bytes(&self.signature)?;
        Ok(public_key.verify_strict(&self.hash(), &signature)?)
    }
    pub fn is_valid(&self) -> bool {
        // check if output is a valid ed25519 public key
        // strictly verify transaction signature
        types::PublicKey::from_bytes(&self.public_key_output).is_ok()
            && self.verify().is_ok()
            && self.timestamp <= util::timestamp()
            && self.public_key_input != self.public_key_output
            && self.amount != 0
    }
    pub fn put(&self, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::cf_handle_transactions(db)?,
            self.hash(),
            bincode::serialize(&CompressedTransaction::from(self))?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Transaction, Box<dyn Error>> {
        let compressed: CompressedTransaction = bincode::deserialize(
            &db.get_cf(db::cf_handle_transactions(db)?, hash)?
                .ok_or("transaction not found")?,
        )?;
        Ok(Transaction::from(&compressed))
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionHeader {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: types::Amount,
    pub fee: types::Amount,
    pub timestamp: types::Timestamp,
}
impl TransactionHeader {
    pub fn from(transaction: &Transaction) -> TransactionHeader {
        TransactionHeader {
            public_key_input: transaction.public_key_input,
            public_key_output: transaction.public_key_output,
            amount: transaction.amount,
            fee: transaction.fee,
            timestamp: transaction.timestamp,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompressedTransaction {
    pub public_key_input: types::PublicKeyBytes,
    pub public_key_output: types::PublicKeyBytes,
    pub amount: types::CompressedAmount,
    pub fee: types::CompressedAmount,
    pub timestamp: types::Timestamp,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl CompressedTransaction {
    pub fn from(transaction: &Transaction) -> CompressedTransaction {
        CompressedTransaction {
            public_key_input: transaction.public_key_input,
            public_key_output: transaction.public_key_output,
            amount: amount::to_bytes(transaction.amount),
            fee: amount::to_bytes(transaction.fee),
            timestamp: transaction.timestamp,
            signature: transaction.signature,
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
        let compressed = CompressedTransaction::from(&transaction);
        println!("{:?}", compressed);
        println!("{:?}", bincode::serialize(&compressed));
        println!("{:?}", bincode::serialize(&compressed).unwrap().len());
        b.iter(|| bincode::serialize(&compressed));
    }
    #[bench]
    fn bench_bincode_deserialize(b: &mut Bencher) {
        let keypair = util::keygen();
        let mut transaction = Transaction::new([0; 32], 0, 0);
        transaction.sign(&keypair);
        let compressed = CompressedTransaction::from(&transaction);
        let bytes = bincode::serialize(&compressed).unwrap();
        b.iter(|| {
            let _: CompressedTransaction = bincode::deserialize(&bytes).unwrap();
        });
    }
}
