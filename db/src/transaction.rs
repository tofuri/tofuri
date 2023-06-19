use crate::Error;
use rocksdb::DB;
use tofuri_transaction::Transaction;
use tracing::instrument;
#[instrument(skip_all, level = "trace")]
pub fn put(transaction_a: &Transaction, db: &DB) -> Result<(), Error> {
    let key = transaction_a.hash();
    let value = bincode::serialize(&transaction_a).map_err(Error::Bincode)?;
    db.put_cf(crate::transactions(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DB, hash: &[u8]) -> Result<Transaction, Error> {
    let key = hash;
    let vec = db
        .get_cf(crate::transactions(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
#[test]
fn test_serialize_len() {
    assert_eq!(
        96,
        bincode::serialize(&Transaction::default()).unwrap().len()
    );
}
