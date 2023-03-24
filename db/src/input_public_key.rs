use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tofuri_core::*;
#[derive(Debug)]
pub enum Error {
    RocksDB(rocksdb::Error),
    NotFound,
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn put(hash: &[u8], input_public_key: &PublicKeyBytes, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
    let key = hash;
    let value = input_public_key;
    db.put_cf(crate::input_public_keys(db), key, value).map_err(Error::RocksDB)
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<PublicKeyBytes, Error> {
    let input_public_key = db.get_cf(crate::input_public_keys(db), hash).map_err(Error::RocksDB)?.ok_or(Error::NotFound)?;
    Ok(input_public_key.try_into().unwrap())
}