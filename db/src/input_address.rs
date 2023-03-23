use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tofuri_core::*;
#[derive(Debug)]
pub enum Error {
    RocksDB(rocksdb::Error),
    NotFound,
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn put(hash: &[u8], input_address: &AddressBytes, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
    let key = hash;
    let value = input_address;
    db.put_cf(crate::input_addresses(db), key, value).map_err(Error::RocksDB)
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<AddressBytes, Error> {
    let input_address = db.get_cf(crate::input_addresses(db), hash).map_err(Error::RocksDB)?.ok_or(Error::NotFound)?;
    Ok(input_address.try_into().unwrap())
}
