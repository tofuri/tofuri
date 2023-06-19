use crate::Error;
use rocksdb::DB;
use tofuri_checkpoint::Checkpoint;
use tracing::instrument;
#[instrument(skip_all, level = "trace")]
pub fn put(db: &DB, checkpoint: &Checkpoint) -> Result<(), Error> {
    let key = [];
    let value = bincode::serialize(checkpoint).map_err(Error::Bincode)?;
    db.put_cf(crate::checkpoint(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DB) -> Result<Checkpoint, Error> {
    let key = [];
    let vec = db
        .get_cf(crate::checkpoint(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
