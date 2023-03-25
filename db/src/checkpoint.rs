use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tofuri_checkpoint::Checkpoint;
#[derive(Debug)]
pub enum Error {
    RocksDB(rocksdb::Error),
    Bincode(bincode::Error),
    NotFound,
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn put(db: &DBWithThreadMode<SingleThreaded>, checkpoint: &Checkpoint) -> Result<(), Error> {
    let key = [];
    let value = bincode::serialize(checkpoint).map_err(Error::Bincode)?;
    db.put_cf(crate::checkpoint(db), key, value)
        .map_err(Error::RocksDB)
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn get(db: &DBWithThreadMode<SingleThreaded>) -> Result<Checkpoint, Error> {
    let key = [];
    let vec = db
        .get_cf(crate::checkpoint(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
