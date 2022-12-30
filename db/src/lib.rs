use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, DBWithThreadMode, Options, SingleThreaded, DB};
fn descriptors() -> Vec<ColumnFamilyDescriptor> {
    let mut options = Options::default();
    options.set_max_write_buffer_number(16);
    vec![
        ColumnFamilyDescriptor::new("blocks", options.clone()),
        ColumnFamilyDescriptor::new("transactions", options.clone()),
        ColumnFamilyDescriptor::new("stakes", options.clone()),
        ColumnFamilyDescriptor::new("stakers", options.clone()),
        ColumnFamilyDescriptor::new("peers", options),
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
pub fn peers(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("peers").unwrap()
}
pub fn inputs(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("inputs").unwrap() // hash, input_address
}
pub fn betas(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("betas").unwrap() // hash, gamma -> beta
}
pub mod block {
    use super::{stake, transaction};
    use pea_block::{BlockA, BlockB, BlockC};
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use std::error::Error;
    pub fn put(block_a: &BlockA, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        for transaction_a in block_a.transactions.iter() {
            transaction::put(transaction_a, db)?;
        }
        for stake_a in block_a.stakes.iter() {
            stake::put(stake_a, db)?;
        }
        db.put_cf(super::blocks(db), &block_a.hash, bincode::serialize(&block_a.b().c())?)?;
        Ok(())
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<BlockB, Box<dyn Error>> {
        let block_c: BlockC = bincode::deserialize(&db.get_cf(super::blocks(db), hash)?.ok_or("block not found")?)?;
        let mut transactions = vec![];
        for hash in block_c.transaction_hashes.iter() {
            transactions.push(transaction::get(db, hash)?);
        }
        let mut stakes = vec![];
        for hash in block_c.stake_hashes.iter() {
            stakes.push(stake::get(db, hash)?);
        }
        Ok(block_c.b(transactions, stakes))
    }
}
pub mod transaction {
    use pea_transaction::{TransactionA, TransactionB, TransactionC};
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use std::error::Error;
    pub fn put(transaction_a: &TransactionA, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(super::transactions(db), transaction_a.hash, bincode::serialize(&transaction_a.b().c())?)?;
        Ok(())
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<TransactionB, Box<dyn Error>> {
        let transaction_c: TransactionC = bincode::deserialize(&db.get_cf(super::transactions(db), hash)?.ok_or("transaction not found")?)?;
        Ok(transaction_c.b())
    }
}
pub mod stake {
    use pea_stake::{StakeA, StakeB, StakeC};
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use std::error::Error;
    pub fn put(stake_a: &StakeA, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(super::stakes(db), stake_a.hash, bincode::serialize(&stake_a.b().c())?)?;
        Ok(())
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<StakeB, Box<dyn Error>> {
        let stake_c: StakeC = bincode::deserialize(&db.get_cf(super::stakes(db), hash)?.ok_or("stake not found")?)?;
        Ok(stake_c.b())
    }
}
pub mod tree {
    use pea_block::BlockC;
    use pea_core::types;
    use pea_tree::Tree;
    use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
    use std::collections::HashMap;
    pub fn reload(tree: &mut Tree, db: &DBWithThreadMode<SingleThreaded>) {
        tree.clear();
        let mut map: HashMap<types::Hash, Vec<(types::Hash, u32)>> = HashMap::new();
        for res in db.iterator_cf(super::blocks(db), IteratorMode::Start) {
            let (hash, bytes) = res.unwrap();
            let hash = hash.to_vec().try_into().unwrap();
            let block_metadata: BlockC = bincode::deserialize(&bytes).unwrap();
            match map.get(&block_metadata.previous_hash) {
                Some(vec) => {
                    let mut vec = vec.clone();
                    vec.push((hash, block_metadata.timestamp));
                    map.insert(block_metadata.previous_hash, vec);
                }
                None => {
                    map.insert(block_metadata.previous_hash, vec![(hash, block_metadata.timestamp)]);
                }
            };
        }
        if map.is_empty() {
            return;
        }
        let previous_hash = [0; 32];
        let mut previous_hashes = vec![previous_hash];
        let mut hashes_0 = vec![];
        for (hash, timestamp) in map.get(&previous_hash).expect("genesis block hashes") {
            hashes_0.push((*hash, *timestamp));
        }
        let mut vec = vec![];
        loop {
            let mut hashes_1 = vec![];
            for previous_hash in previous_hashes.clone() {
                for (hash, timestamp) in hashes_0.clone() {
                    vec.push((hash, previous_hash, timestamp));
                    if let Some(vec) = map.remove(&hash) {
                        for (hash, timestamp) in vec {
                            hashes_1.push((hash, timestamp));
                        }
                    };
                }
            }
            if hashes_1.is_empty() {
                break;
            }
            previous_hashes.clear();
            for (hash, _) in hashes_0 {
                previous_hashes.push(hash);
            }
            hashes_0 = hashes_1;
        }
        for (hash, previous_hash, timestamp) in vec {
            tree.insert(hash, previous_hash, timestamp);
        }
        tree.sort_branches();
    }
}
pub mod peer {
    use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
    use std::error::Error;
    pub fn put(peer: &str, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(super::peers(db), peer, &[])?;
        Ok(())
    }
    pub fn get_all(db: &DBWithThreadMode<SingleThreaded>) -> Vec<String> {
        let mut peers: Vec<String> = vec![];
        for res in db.iterator_cf(super::peers(db), IteratorMode::Start) {
            let (peer, _) = res.unwrap();
            peers.push(std::str::from_utf8(&peer).unwrap().to_string());
        }
        peers
    }
}
