use crate::{db, types, util};
use ed25519::signature::Signer;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Input {
    pub public_key: types::PublicKeyBytes,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Input {
    pub fn new(hash: &types::Hash, keypair: &types::Keypair) -> Input {
        Input {
            public_key: keypair.public.to_bytes(),
            signature: keypair.sign(hash).to_bytes(),
        }
    }
    pub fn verify(&mut self, hash: &types::Hash) -> Result<(), Box<dyn Error>> {
        let public_key = types::PublicKey::from_bytes(&self.public_key)?;
        let signature = types::Signature::from_bytes(&self.signature)?;
        Ok(public_key.verify_strict(hash, &signature)?)
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Output {
    pub public_key: types::PublicKeyBytes,
    pub amount: types::Amount,
}
impl Output {
    fn new(public_key: types::PublicKeyBytes, amount: types::Amount) -> Output {
        Output { public_key, amount }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub timestamp: types::Timestamp,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}
impl Transaction {
    pub fn new(outputs: Vec<Output>, keypairs: &[types::Keypair]) -> Transaction {
        let timestamp = util::timestamp();
        let mut transaction = Transaction {
            timestamp,
            inputs: vec![],
            outputs,
        };
        let hash = transaction.hash();
        transaction.inputs = keypairs.iter().map(|k| Input::new(&hash, k)).collect();
        transaction
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        let hash = self.hash();
        for input in self.inputs {
            input.verify(&hash)?;
        }
        Ok(())
    }
    fn hash(&self) -> types::Hash {
        #[derive(Serialize)]
        pub struct Data {
            pub timestamp: types::Timestamp,
            pub outputs: Vec<Output>,
        }
        util::hash(
            &bincode::serialize(&Data {
                timestamp: self.timestamp,
                outputs: self.outputs,
            })
            .unwrap(),
        )
    }
    pub fn is_valid(&self) -> bool {
        self.timestamp <= util::timestamp() && self.verify().is_ok()
    }
    pub fn put(&self, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::cf_handle_transactions(db)?,
            self.hash(),
            bincode::serialize(&self)?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Transaction, Box<dyn Error>> {
        let transaction: Transaction = bincode::deserialize(
            &db.get_cf(db::cf_handle_transactions(db)?, hash)?
                .ok_or("transaction not found")?,
        )?;
        Ok(transaction)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[bench]
    fn bench_hash(b: &mut Bencher) {
        let keypair = util::keygen();
        b.iter(|| Transaction::new(vec![Output::new([0; 32], 0)], &[keypair]));
    }
}
