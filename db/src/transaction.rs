use crate::Error;
use rocksdb::ColumnFamily;
use rocksdb::DB;
use tofuri_transaction::Transaction;
use tracing::instrument;
pub fn cf_handle(db: &DB) -> &ColumnFamily {
    db.cf_handle("transaction").unwrap()
}
#[instrument(skip_all, level = "trace")]
pub fn put(transaction: &Transaction, db: &DB) -> Result<(), Error> {
    let key = transaction.hash();
    let value = bincode::serialize(&transaction).map_err(Error::Bincode)?;
    db.put_cf(cf_handle(db), key, value).map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DB, hash: &[u8]) -> Result<Transaction, Error> {
    let key = hash;
    let vec = db
        .get_cf(cf_handle(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
