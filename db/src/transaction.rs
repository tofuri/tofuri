use crate::Error;
use rocksdb::ColumnFamily;
use rocksdb::DB;
use tracing::instrument;
use transaction::Transaction;
pub fn cf(db: &DB) -> &ColumnFamily {
    db.cf_handle("transaction").unwrap()
}
#[instrument(skip_all, level = "trace")]
pub fn put(db: &DB, transaction: &Transaction) -> Result<(), Error> {
    let key = transaction.hash();
    let value = bincode::serialize(&transaction).map_err(Error::Bincode)?;
    db.put_cf(cf(db), key, value).map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DB, hash: &[u8]) -> Result<Transaction, Error> {
    let key = hash;
    let vec = db
        .get_cf(cf(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
