use rocksdb::{
    ColumnFamily, ColumnFamilyDescriptor, DBWithThreadMode, Options, SingleThreaded, DB,
};
pub enum Key {
    LatestBlockHash,
}
pub fn key(key: &Key) -> &[u8] {
    match *key {
        Key::LatestBlockHash => &[0],
    }
}
fn descriptors() -> Vec<ColumnFamilyDescriptor> {
    let mut options = Options::default();
    options.set_max_write_buffer_number(16);
    vec![
        ColumnFamilyDescriptor::new("blocks", options.clone()),
        ColumnFamilyDescriptor::new("transactions", options.clone()),
        ColumnFamilyDescriptor::new("stakes", options.clone()),
        ColumnFamilyDescriptor::new("stakers", options),
    ]
}
pub fn open(path: &str) -> DBWithThreadMode<SingleThreaded> {
    let mut options = Options::default();
    options.create_missing_column_families(true);
    options.create_if_missing(true);
    DB::open_cf_descriptors(&options, path, descriptors()).unwrap()
}
pub fn blocks(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("blocks").unwrap()
}
pub fn transactions(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("transactions").unwrap()
}
pub fn stakes(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("stakes").unwrap()
}
pub fn stakers(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("stakers").unwrap()
}
pub mod block {
    use super::block_metadata_lean;
    use super::stake;
    use super::transaction;
    use pea_core::block::{self, Block};
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use std::error::Error;
    pub fn put(block: &Block, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        let block_metadata = block::Metadata::from(block);
        let block_metadata_lean = block::MetadataLean::from(&block_metadata);
        for transaction in block.transactions.iter() {
            transaction::put(transaction, db)?;
        }
        for stake in block.stakes.iter() {
            stake::put(stake, db)?;
        }
        block_metadata_lean::put(db, &block.hash(), block_metadata_lean)?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Block, Box<dyn Error>> {
        let block_metadata_lean: block::MetadataLean =
            bincode::deserialize(&block_metadata_lean::get(db, hash)?)?;
        let mut transactions = vec![];
        for hash in block_metadata_lean.transaction_hashes {
            transactions.push(transaction::get(db, &hash)?);
        }
        let mut stakes = vec![];
        for hash in block_metadata_lean.stake_hashes {
            stakes.push(stake::get(db, &hash)?);
        }
        Ok(Block::from(
            block_metadata_lean.previous_hash,
            block_metadata_lean.timestamp,
            block_metadata_lean.public_key,
            block_metadata_lean.signature,
            transactions,
            stakes,
        ))
    }
}
pub mod block_metadata_lean {
    use pea_core::{
        block::{self},
        types,
    };
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use std::error::Error;
    pub fn put(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &types::Hash,
        block_metadata_lean: block::MetadataLean,
    ) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            super::blocks(db),
            hash,
            bincode::serialize(&block_metadata_lean)?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(db
            .get_cf(super::blocks(db), hash)?
            .ok_or("block not found")?)
    }
}
pub mod transaction {
    use pea_core::transaction::{self, Transaction};
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use std::error::Error;
    pub fn put(
        transaction: &Transaction,
        db: &DBWithThreadMode<SingleThreaded>,
    ) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            super::transactions(db),
            transaction.hash(),
            bincode::serialize(&transaction::Compressed {
                public_key_input: transaction.public_key_input,
                public_key_output: transaction.public_key_output,
                amount: pea_amount::to_bytes(&transaction.amount),
                fee: pea_amount::to_bytes(&transaction.fee),
                timestamp: transaction.timestamp,
                signature: transaction.signature,
            })?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Transaction, Box<dyn Error>> {
        let compressed: transaction::Compressed = bincode::deserialize(
            &db.get_cf(super::transactions(db), hash)?
                .ok_or("transaction not found")?,
        )?;
        Ok(Transaction {
            public_key_input: compressed.public_key_input,
            public_key_output: compressed.public_key_output,
            amount: pea_amount::from_bytes(&compressed.amount),
            fee: pea_amount::from_bytes(&compressed.fee),
            timestamp: compressed.timestamp,
            signature: compressed.signature,
        })
    }
}
pub mod stake {
    use pea_core::stake::{self, Stake};
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use std::error::Error;
    pub fn put(stake: &Stake, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            super::stakes(db),
            stake.hash(),
            bincode::serialize(&stake::Compressed {
                public_key: stake.public_key,
                amount: pea_amount::to_bytes(&stake.amount),
                fee: pea_amount::to_bytes(&stake.fee),
                deposit: stake.deposit,
                timestamp: stake.timestamp,
                signature: stake.signature,
            })?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Stake, Box<dyn Error>> {
        let compressed: stake::Compressed = bincode::deserialize(
            &db.get_cf(super::stakes(db), hash)?
                .ok_or("stake not found")?,
        )?;
        Ok(Stake {
            public_key: compressed.public_key,
            amount: pea_amount::from_bytes(&compressed.amount),
            fee: pea_amount::from_bytes(&compressed.fee),
            deposit: compressed.deposit,
            timestamp: compressed.timestamp,
            signature: compressed.signature,
        })
    }
}
pub mod tree {
    use pea_core::{block::MetadataLean, types};
    use pea_tree::Tree;
    use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
    use std::collections::HashMap;
    pub fn reload(tree: &mut Tree, db: &DBWithThreadMode<SingleThreaded>) {
        tree.clear();
        let mut hashes: HashMap<types::Hash, (Vec<types::Hash>, types::Timestamp)> = HashMap::new();
        for res in db.iterator_cf(super::blocks(db), IteratorMode::Start) {
            let (hash, bytes) = res.unwrap();
            let hash = hash.to_vec().try_into().unwrap();
            let block: MetadataLean = bincode::deserialize(&bytes).unwrap();
            match hashes.get(&block.previous_hash) {
                Some((vec, _)) => {
                    let mut vec = vec.clone();
                    vec.push(hash);
                    hashes.insert(block.previous_hash, (vec, block.timestamp));
                }
                None => {
                    hashes.insert(block.previous_hash, (vec![hash], block.timestamp));
                }
            };
        }
        if hashes.is_empty() {
            return;
        }
        let previous_hash = [0; 32];
        let (_, (vec, timestamp)) = hashes.iter().find(|(&x, _)| x == previous_hash).unwrap();
        fn recurse(
            tree: &mut Tree,
            hashes: &HashMap<types::Hash, (Vec<types::Hash>, types::Timestamp)>,
            previous_hash: types::Hash,
            vec: &Vec<types::Hash>,
            timestamp: types::Timestamp,
        ) {
            for hash in vec {
                tree.insert(*hash, previous_hash, timestamp);
                if let Some((vec, timestamp)) = hashes.get(hash) {
                    recurse(tree, hashes, *hash, vec, *timestamp);
                };
            }
        }
        recurse(tree, &hashes, previous_hash, vec, *timestamp);
        tree.sort_branches();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;
    use test::Bencher;
    #[bench]
    fn bench_put(b: &mut Bencher) {
        let tempdir = TempDir::new("rocksdb").unwrap();
        let db = open(tempdir.path().to_str().unwrap());
        b.iter(|| db.put(b"test", b"value"));
    }
    #[bench]
    fn bench_get(b: &mut Bencher) {
        let tempdir = TempDir::new("rocksdb").unwrap();
        let db = open(tempdir.path().to_str().unwrap());
        b.iter(|| db.get(b"test"));
    }
}
