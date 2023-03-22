use rocksdb::ColumnFamily;
use rocksdb::ColumnFamilyDescriptor;
use rocksdb::DBWithThreadMode;
use rocksdb::Options;
use rocksdb::SingleThreaded;
use rocksdb::DB;
use std::path::Path;
#[derive(Debug)]
pub enum Error {
    Block(tofuri_block::Error),
    Transaction(tofuri_transaction::Error),
    Stake(tofuri_stake::Error),
    RocksDB(rocksdb::Error),
    Bincode(bincode::Error),
    BlockNotFound,
    TransactionNotFound,
    StakeNotFound,
    InputAddressNotFound,
    InputPublicKeyNotFound,
    BetaNotFound,
    CheckpointNotFound,
}
fn descriptors() -> Vec<ColumnFamilyDescriptor> {
    let options = Options::default();
    vec![
        ColumnFamilyDescriptor::new("blocks", options.clone()),
        ColumnFamilyDescriptor::new("transactions", options.clone()),
        ColumnFamilyDescriptor::new("stakes", options.clone()),
        ColumnFamilyDescriptor::new("peers", options.clone()),
        ColumnFamilyDescriptor::new("input addresses", options.clone()),
        ColumnFamilyDescriptor::new("input public keys", options.clone()),
        ColumnFamilyDescriptor::new("betas", options.clone()),
        ColumnFamilyDescriptor::new("checkpoint", options),
    ]
}
pub fn open(path: impl AsRef<Path>) -> DBWithThreadMode<SingleThreaded> {
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
pub fn peers(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("peers").unwrap()
}
pub fn input_addresses(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("input addresses").unwrap()
}
pub fn input_public_keys(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("input public keys").unwrap()
}
pub fn betas(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("betas").unwrap()
}
pub fn checkpoint(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("checkpoint").unwrap()
}
pub mod block {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::SingleThreaded;
    use tofuri_block::BlockA;
    use tofuri_block::BlockB;
    use tofuri_block::BlockC;
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn put(block_a: &BlockA, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
        for transaction_a in block_a.transactions.iter() {
            transaction::put(transaction_a, db)?;
        }
        for stake_a in block_a.stakes.iter() {
            stake::put(stake_a, db)?;
        }
        let key = block_a.hash;
        let value = bincode::serialize(&block_a.b().c()).map_err(Error::Bincode)?;
        db.put_cf(super::blocks(db), key, value).map_err(Error::RocksDB)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_a(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<BlockA, Error> {
        let block_c = get_c(db, hash)?;
        let mut transactions = vec![];
        let mut stakes = vec![];
        for hash in block_c.transaction_hashes.iter() {
            transactions.push(transaction::get_a(db, hash)?);
        }
        for hash in block_c.stake_hashes.iter() {
            stakes.push(stake::get_a(db, hash)?);
        }
        let beta = beta::get(db, hash).ok();
        let input_public_key = input_public_key::get(db, hash).ok();
        let block_a = block_c.a(transactions, stakes, beta, input_public_key).map_err(Error::Block)?;
        if beta.is_none() {
            beta::put(hash, &block_a.beta, db)?;
        }
        if input_public_key.is_none() {
            input_public_key::put(hash, &block_a.input_public_key, db)?;
        }
        Ok(block_a)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_b(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<BlockB, Error> {
        let block_c = get_c(db, hash)?;
        let mut transactions = vec![];
        for hash in block_c.transaction_hashes.iter() {
            transactions.push(transaction::get_b(db, hash)?);
        }
        let mut stakes = vec![];
        for hash in block_c.stake_hashes.iter() {
            stakes.push(stake::get_b(db, hash)?);
        }
        Ok(block_c.b(transactions, stakes))
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_c(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<BlockC, Error> {
        let key = hash;
        let vec = db.get_cf(super::blocks(db), key).map_err(Error::RocksDB)?.ok_or(Error::BlockNotFound)?;
        Ok(bincode::deserialize(&vec).map_err(Error::Bincode)?)
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(197, bincode::serialize(&BlockC::default()).unwrap().len());
    }
}
pub mod transaction {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::SingleThreaded;
    use tofuri_transaction::TransactionA;
    use tofuri_transaction::TransactionB;
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn put(transaction_a: &TransactionA, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
        let key = transaction_a.hash;
        let value = bincode::serialize(&transaction_a.b()).map_err(Error::Bincode)?;
        db.put_cf(super::transactions(db), key, value).map_err(Error::RocksDB)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_a(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<TransactionA, Error> {
        let input_address = input_address::get(db, hash).ok();
        let transaction_a = get_b(db, hash)?.a(input_address).map_err(Error::Transaction)?;
        if input_address.is_none() {
            input_address::put(hash, &transaction_a.input_address, db)?;
        }
        Ok(transaction_a)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_b(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<TransactionB, Error> {
        let key = hash;
        let vec = db
            .get_cf(super::transactions(db), key)
            .map_err(Error::RocksDB)?
            .ok_or(Error::TransactionNotFound)?;
        Ok(bincode::deserialize(&vec).map_err(Error::Bincode)?)
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(96, bincode::serialize(&TransactionB::default()).unwrap().len());
    }
}
pub mod stake {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::SingleThreaded;
    use tofuri_stake::StakeA;
    use tofuri_stake::StakeB;
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn put(stake_a: &StakeA, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
        let key = stake_a.hash;
        let value = bincode::serialize(&stake_a.b()).map_err(Error::Bincode)?;
        db.put_cf(super::stakes(db), key, value).map_err(Error::RocksDB)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_a(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<StakeA, Error> {
        let input_address = input_address::get(db, hash).ok();
        let stake_a = get_b(db, hash)?.a(input_address).map_err(Error::Stake)?;
        if input_address.is_none() {
            input_address::put(hash, &stake_a.input_address, db)?;
        }
        Ok(stake_a)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_b(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<StakeB, Error> {
        let key = hash;
        let vec = db.get_cf(super::stakes(db), key).map_err(Error::RocksDB)?.ok_or(Error::StakeNotFound)?;
        Ok(bincode::deserialize(&vec).map_err(Error::Bincode)?)
    }
    #[test]
    fn test_serialize_len() {
        assert_eq!(77, bincode::serialize(&StakeB::default()).unwrap().len());
    }
}
pub mod tree {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::IteratorMode;
    use rocksdb::SingleThreaded;
    use std::collections::HashMap;
    use tofuri_block::BlockC;
    use tofuri_core::*;
    use tofuri_tree::Tree;
    #[tracing::instrument(skip_all, level = "debug")]
    pub fn reload(tree: &mut Tree, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
        tree.clear();
        let mut map: HashMap<Hash, Vec<(Hash, u32)>> = HashMap::new();
        for res in db.iterator_cf(super::blocks(db), IteratorMode::Start) {
            let (hash, bytes) = res.map_err(Error::RocksDB)?;
            let hash = hash.to_vec().try_into().unwrap();
            let block_metadata: BlockC = bincode::deserialize(&bytes).map_err(Error::Bincode)?;
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
            return Ok(());
        }
        let previous_hash = GENESIS_BLOCK_PREVIOUS_HASH;
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
        Ok(())
    }
}
pub mod peer {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::IteratorMode;
    use rocksdb::SingleThreaded;
    use std::net::IpAddr;
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn put(ip_addr: &IpAddr, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
        let key = bincode::serialize(ip_addr).map_err(Error::Bincode)?;
        let value = [];
        db.put_cf(super::peers(db), key, value).map_err(Error::RocksDB)
    }
    #[tracing::instrument(skip_all, level = "debug")]
    pub fn get_all(db: &DBWithThreadMode<SingleThreaded>) -> Result<Vec<IpAddr>, Error> {
        let mut peers: Vec<IpAddr> = vec![];
        for res in db.iterator_cf(super::peers(db), IteratorMode::Start) {
            let (peer, _) = res.map_err(Error::RocksDB)?;
            peers.push(bincode::deserialize(&peer).map_err(Error::Bincode)?);
        }
        Ok(peers)
    }
}
pub mod input_address {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::SingleThreaded;
    use tofuri_core::*;
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn put(hash: &[u8], input_address: &AddressBytes, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
        let key = hash;
        let value = input_address;
        db.put_cf(super::input_addresses(db), key, value).map_err(Error::RocksDB)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<AddressBytes, Error> {
        let input_address = db
            .get_cf(super::input_addresses(db), hash)
            .map_err(Error::RocksDB)?
            .ok_or(Error::InputAddressNotFound)?;
        Ok(input_address.try_into().unwrap())
    }
}
pub mod input_public_key {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::SingleThreaded;
    use tofuri_core::*;
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn put(hash: &[u8], input_public_key: &PublicKeyBytes, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
        let key = hash;
        let value = input_public_key;
        db.put_cf(super::input_public_keys(db), key, value).map_err(Error::RocksDB)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<PublicKeyBytes, Error> {
        let input_public_key = db
            .get_cf(super::input_public_keys(db), hash)
            .map_err(Error::RocksDB)?
            .ok_or(Error::InputPublicKeyNotFound)?;
        Ok(input_public_key.try_into().unwrap())
    }
}
pub mod beta {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::SingleThreaded;
    use tofuri_core::*;
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn put(block_hash: &[u8], beta: &Beta, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
        let key = block_hash;
        let value = beta;
        db.put_cf(super::betas(db), key, value).map_err(Error::RocksDB)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, block_hash: &[u8]) -> Result<Beta, Error> {
        let key = block_hash;
        let vec = db.get_cf(super::betas(db), key).map_err(Error::RocksDB)?.ok_or(Error::BetaNotFound)?;
        Ok(vec.try_into().unwrap())
    }
}
pub mod checkpoint {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::SingleThreaded;
    use tofuri_checkpoint::Checkpoint;
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn put(db: &DBWithThreadMode<SingleThreaded>, checkpoint: &Checkpoint) -> Result<(), Error> {
        let key = [];
        let value = bincode::serialize(checkpoint).map_err(Error::Bincode)?;
        db.put_cf(super::checkpoint(db), key, value).map_err(Error::RocksDB)
    }
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get(db: &DBWithThreadMode<SingleThreaded>) -> Result<Checkpoint, Error> {
        let key = [];
        let vec = db
            .get_cf(super::checkpoint(db), key)
            .map_err(Error::RocksDB)?
            .ok_or(Error::CheckpointNotFound)?;
        Ok(bincode::deserialize(&vec).map_err(Error::Bincode)?)
    }
}
