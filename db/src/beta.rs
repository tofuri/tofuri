use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tofuri_core::*;
#[derive(Debug)]
pub enum Error {
    RocksDB(rocksdb::Error),
    NotFound,
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn put(
    block_hash: &[u8],
    beta: &Beta,
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<(), Error> {
    let key = block_hash;
    let value = beta;
    db.put_cf(crate::betas(db), key, value)
        .map_err(Error::RocksDB)
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn get(db: &DBWithThreadMode<SingleThreaded>, block_hash: &[u8]) -> Result<Beta, Error> {
    let key = block_hash;
    let vec = db
        .get_cf(crate::betas(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    Ok(vec.try_into().unwrap())
}
