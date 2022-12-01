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
pub mod block {
    use super::{stake, transaction};
    use pea_block::Block;
    use pea_core::types;
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use serde::{Deserialize, Serialize};
    use serde_big_array::BigArray;
    use std::error::Error;
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Metadata {
        pub previous_hash: types::Hash,
        pub timestamp: u32,
        pub public_key: types::PublicKeyBytes,
        #[serde(with = "BigArray")]
        pub signature: types::SignatureBytes,
        pub transaction_hashes: Vec<types::Hash>,
        pub stake_hashes: Vec<types::Hash>,
    }
    pub fn put(block: &Block, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        for transaction in block.transactions.iter() {
            transaction::put(transaction, db)?;
        }
        for stake in block.stakes.iter() {
            stake::put(stake, db)?;
        }
        db.put_cf(
            super::blocks(db),
            &block.hash(),
            bincode::serialize(&Metadata {
                previous_hash: block.previous_hash,
                timestamp: block.timestamp,
                public_key: block.public_key,
                signature: block.signature,
                transaction_hashes: block.transaction_hashes(),
                stake_hashes: block.stake_hashes(),
            })?,
        )?;
        Ok(())
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<Block, Box<dyn Error>> {
        let metadata: Metadata = bincode::deserialize(&db.get_cf(super::blocks(db), hash)?.ok_or("block not found")?)?;
        let mut transactions = vec![];
        for hash in metadata.transaction_hashes {
            transactions.push(transaction::get(db, &hash)?);
        }
        let mut stakes = vec![];
        for hash in metadata.stake_hashes {
            stakes.push(stake::get(db, &hash)?);
        }
        Ok(Block {
            previous_hash: metadata.previous_hash,
            timestamp: metadata.timestamp,
            public_key: metadata.public_key,
            signature: metadata.signature,
            transactions,
            stakes,
        })
    }
}
pub mod transaction {
    use pea_core::types;
    use pea_transaction::Transaction;
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use serde::{Deserialize, Serialize};
    use serde_big_array::BigArray;
    use std::error::Error;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Metadata {
        pub public_key_input: types::PublicKeyBytes,
        pub public_key_output: types::PublicKeyBytes,
        pub amount: types::CompressedAmount,
        pub fee: types::CompressedAmount,
        pub timestamp: u32,
        #[serde(with = "BigArray")]
        pub signature: types::SignatureBytes,
    }
    pub fn put(transaction: &Transaction, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            super::transactions(db),
            transaction.hash(),
            bincode::serialize(&Metadata {
                public_key_input: transaction.public_key_input,
                public_key_output: transaction.public_key_output,
                amount: pea_int::to_bytes(transaction.amount),
                fee: pea_int::to_bytes(transaction.fee),
                timestamp: transaction.timestamp,
                signature: transaction.signature,
            })?,
        )?;
        Ok(())
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<Transaction, Box<dyn Error>> {
        let metadata: Metadata = bincode::deserialize(&db.get_cf(super::transactions(db), hash)?.ok_or("transaction not found")?)?;
        Ok(Transaction {
            public_key_input: metadata.public_key_input,
            public_key_output: metadata.public_key_output,
            amount: pea_int::from_bytes(&metadata.amount),
            fee: pea_int::from_bytes(&metadata.fee),
            timestamp: metadata.timestamp,
            signature: metadata.signature,
        })
    }
}
pub mod stake {
    use pea_core::types;
    use pea_stake::Stake;
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use serde::{Deserialize, Serialize};
    use serde_big_array::BigArray;
    use std::error::Error;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Metadata {
        pub public_key: types::PublicKeyBytes,
        pub amount: types::CompressedAmount,
        pub fee: types::CompressedAmount,
        pub deposit: bool,
        pub timestamp: u32,
        #[serde(with = "BigArray")]
        pub signature: types::SignatureBytes,
    }
    pub fn put(stake: &Stake, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            super::stakes(db),
            stake.hash(),
            bincode::serialize(&Metadata {
                public_key: stake.public_key,
                amount: pea_int::to_bytes(stake.amount),
                fee: pea_int::to_bytes(stake.fee),
                deposit: stake.deposit,
                timestamp: stake.timestamp,
                signature: stake.signature,
            })?,
        )?;
        Ok(())
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<Stake, Box<dyn Error>> {
        let metadata: Metadata = bincode::deserialize(&db.get_cf(super::stakes(db), hash)?.ok_or("stake not found")?)?;
        Ok(Stake {
            public_key: metadata.public_key,
            amount: pea_int::from_bytes(&metadata.amount),
            fee: pea_int::from_bytes(&metadata.fee),
            deposit: metadata.deposit,
            timestamp: metadata.timestamp,
            signature: metadata.signature,
        })
    }
}
pub mod tree {
    use super::block;
    use pea_core::types;
    use pea_tree::Tree;
    use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
    use std::collections::HashMap;
    pub fn reload(tree: &mut Tree, db: &DBWithThreadMode<SingleThreaded>) {
        tree.clear();
        let mut map: HashMap<types::Hash, (Vec<types::Hash>, u32)> = HashMap::new();
        for res in db.iterator_cf(super::blocks(db), IteratorMode::Start) {
            let (hash, bytes) = res.unwrap();
            let hash = hash.to_vec().try_into().unwrap();
            let block: block::Metadata = bincode::deserialize(&bytes).unwrap();
            match map.get(&block.previous_hash) {
                Some((vec, _)) => {
                    let mut vec = vec.clone();
                    vec.push(hash);
                    map.insert(block.previous_hash, (vec, block.timestamp));
                }
                None => {
                    map.insert(block.previous_hash, (vec![hash], block.timestamp));
                }
            };
        }
        if map.is_empty() {
            return;
        }
        let previous_hash = [0; 32];
        let mut previous_hashes = vec![previous_hash];
        let mut hashes_0 = vec![];
        let (_, (genesis_hashes, timestamp)) = map.iter().find(|(&x, _)| x == previous_hash).unwrap();
        for &hash in genesis_hashes {
            hashes_0.push((hash, *timestamp));
        }
        let mut vec = vec![];
        loop {
            let mut hashes_1 = vec![];
            for previous_hash in previous_hashes.clone() {
                for (hash, timestamp) in hashes_0.clone() {
                    vec.push((hash, previous_hash, timestamp));
                    if let Some((hashes, timestamp)) = map.remove(&hash) {
                        for hash in hashes {
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
        // fn recurse(
        // tree: &mut Tree,
        // hashes: &HashMap<types::Hash, (Vec<types::Hash>, u32)>,
        // previous_hash: types::Hash,
        // vec: &Vec<types::Hash>,
        // timestamp: u32,
        // ) {
        // for hash in vec {
        // tree.insert(*hash, previous_hash, timestamp);
        // if let Some((vec, timestamp)) = hashes.get(hash) {
        // recurse(tree, hashes, *hash, vec, *timestamp);
        // };
        // }
        // }
        // recurse(tree, &hashes, previous_hash, vec, *timestamp);
        tree.sort_branches();
    }
}
pub mod peer {
    use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
    use std::error::Error;
    pub fn put(peer: &str, value: &[u8], db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        db.put_cf(super::peers(db), peer, value)?;
        Ok(())
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, peer: &str) -> Result<u32, Box<dyn Error>> {
        let bytes: [u8; 4] = db.get_cf(super::peers(db), peer)?.ok_or("peer not found")?.as_slice().try_into()?;
        Ok(u32::from_le_bytes(bytes))
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
