use crate::Error;
use rocksdb::DB;
use serde::Deserialize;
use serde_big_array::BigArray;
use tracing::instrument;
#[instrument(skip_all, level = "trace")]
pub fn put(hash: &[u8], input_public_key: &[u8; 33], db: &DB) -> Result<(), Error> {
    let key = hash;
    let value = input_public_key;
    db.put_cf(crate::input_public_keys(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DB, hash: &[u8]) -> Result<[u8; 33], Error> {
    let vec = db
        .get_cf(crate::input_public_keys(db), hash)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    let input_public_key: InputPublicKey = bincode::deserialize(&vec).map_err(Error::Bincode)?;
    Ok(input_public_key.0)
}
#[derive(Deserialize)]
struct InputPublicKey(#[serde(with = "BigArray")] pub [u8; 33]);
