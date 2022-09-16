use crate::{db, types, util};
use ed25519::signature::Signer;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Input {
    pub public_key: types::PublicKeyBytes,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
}
impl Input {
    pub fn verify(&mut self, hash: &types::Hash) -> Result<(), Box<dyn Error>> {
        let public_key = types::PublicKey::from_bytes(&self.public_key)?;
        let signature = types::Signature::from_bytes(&self.signature)?;
        Ok(public_key.verify_strict(hash, &signature)?)
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Output {
    pub public_key: types::PublicKeyBytes,
    pub amount: types::Amount,
}
impl Output {
    pub fn put(&self, db: &DBWithThreadMode<SingleThreaded>) {
        let amount = match db.get_cf(db::outputs(db), self.public_key).unwrap() {
            Some(bytes) => bincode::deserialize(&bytes).unwrap(),
            None => 0,
        };
        db.put_cf(
            db::transactions(db),
            self.public_key,
            bincode::serialize(&self.amount).unwrap(),
        )
        .unwrap();
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        public_key: types::PublicKeyBytes,
    ) -> Option<Output> {
        match db.get_cf(db::outputs(db), public_key).unwrap() {
            Some(bytes) => Some(Output {
                public_key,
                amount: bincode::deserialize(&bytes).unwrap(),
            }),
            None => None,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub timestamp: types::Timestamp,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}
impl Transaction {
    pub fn input(hash: &types::Hash, keypair: &types::Keypair) -> Input {
        Input {
            public_key: keypair.public.to_bytes(),
            signature: keypair.sign(hash).to_bytes(),
        }
    }
    pub fn output(public_key: types::PublicKeyBytes, amount: types::Amount) -> Output {
        Output { public_key, amount }
    }
    pub fn new(outputs: Vec<Output>, keypairs: &[&types::Keypair]) -> Transaction {
        let timestamp = util::timestamp();
        let mut transaction = Transaction {
            timestamp,
            inputs: vec![],
            outputs,
        };
        let hash = transaction.hash();
        transaction.inputs = keypairs
            .iter()
            .map(|k| Transaction::input(&hash, k))
            .collect();
        transaction
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        let hash = self.hash();
        for input in self.inputs {
            input.verify(&hash)?;
        }
        Ok(())
    }
    pub fn hash(&self) -> types::Hash {
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
    pub fn sum_inputs(&self, db: &DBWithThreadMode<SingleThreaded>) -> types::Amount {
        let sum = 0;
        for input in self.inputs {
            // find the unspent outputs to the inputs then look at the amounts of the outputs
            Output::get()
            sum += Transaction::get_balance(db, &input.public_key);
        }
        sum
    }
    pub fn sum_outputs(&self) -> types::Amount {
        let sum = 0;
        for output in self.outputs {
            sum += output.amount;
        }
        sum
    }
    pub fn is_valid(&self) -> bool {
        self.timestamp <= util::timestamp() && self.verify().is_ok()
    }
    pub fn put(&self, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::transactions(db),
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
            &db.get_cf(db::transactions(db), hash)?
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
        b.iter(|| Transaction::new(vec![Transaction::output([0; 32], 0)], &[&keypair]));
    }
}
