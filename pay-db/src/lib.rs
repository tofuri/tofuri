use rocksdb::ColumnFamily;
use rocksdb::ColumnFamilyDescriptor;
use rocksdb::DBWithThreadMode;
use rocksdb::Options;
use rocksdb::SingleThreaded;
use rocksdb::DB;
use std::path::Path;
#[derive(Debug)]
pub enum Error {
    RocksDB(rocksdb::Error),
    Bincode(bincode::Error),
    ChargeNotFound,
}
fn descriptors() -> Vec<ColumnFamilyDescriptor> {
    let options = Options::default();
    vec![ColumnFamilyDescriptor::new("charges", options)]
}
pub fn open(path: impl AsRef<Path>) -> DBWithThreadMode<SingleThreaded> {
    let mut options = Options::default();
    options.create_missing_column_families(true);
    options.create_if_missing(true);
    DB::open_cf_descriptors(&options, path, descriptors()).unwrap()
}
pub fn charges(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("charges").unwrap()
}
pub mod charge {
    use super::*;
    use rocksdb::DBWithThreadMode;
    use rocksdb::SingleThreaded;
    use tofuri_key::Key;
    use tofuri_pay_core::Charge;
    pub fn put(db: &DBWithThreadMode<SingleThreaded>, key: &Key, charge: &Charge) -> Result<(), Error> {
        let key = charge.address_bytes(key);
        let value = bincode::serialize(charge).map_err(Error::Bincode)?;
        db.put_cf(super::charges(db), key, value).map_err(Error::RocksDB)
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<Charge, Error> {
        let key = hash;
        let vec = db.get_cf(super::charges(db), key).map_err(Error::RocksDB)?.ok_or(Error::ChargeNotFound)?;
        bincode::deserialize(&vec).map_err(Error::Bincode)
    }
}
