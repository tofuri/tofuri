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
    hash: &[u8],
    input_address: &[u8; 20],
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<(), Error> {
    let key = hash;
    let value = input_address;
    db.put_cf(crate::input_addresses(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<[u8; 20], Error> {
    let vec = db
        .get_cf(crate::input_addresses(db), hash)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    let input_address = vec.try_into().unwrap();
    Ok(input_address)
}
