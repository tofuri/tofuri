use crate::Error;
use rocksdb::DB;
use tofuri_stake::Stake;
use tracing::instrument;
#[instrument(skip_all, level = "trace")]
pub fn put(stake: &Stake, db: &DB) -> Result<(), Error> {
    let key = stake.hash();
    let value = bincode::serialize(&stake).map_err(Error::Bincode)?;
    db.put_cf(crate::stakes(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DB, hash: &[u8]) -> Result<Stake, Error> {
    let key = hash;
    let vec = db
        .get_cf(crate::stakes(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
