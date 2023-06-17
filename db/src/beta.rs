use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tracing::instrument;
#[derive(Debug)]
pub enum Error {
    RocksDB(rocksdb::Error),
    NotFound,
}
#[instrument(skip_all, level = "trace")]
pub fn put(
    block_hash: &[u8],
    beta: &[u8; 32],
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<(), Error> {
    let key = block_hash;
    let value = beta;
    db.put_cf(crate::betas(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DBWithThreadMode<SingleThreaded>, block_hash: &[u8]) -> Result<[u8; 32], Error> {
    let key = block_hash;
    let vec = db
        .get_cf(crate::betas(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    let beta = vec.try_into().unwrap();
    Ok(beta)
}
