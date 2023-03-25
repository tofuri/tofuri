use crate::input_address;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tofuri_transaction::TransactionA;
use tofuri_transaction::TransactionB;
#[derive(Debug)]
pub enum Error {
    Transaction(tofuri_transaction::Error),
    RocksDB(rocksdb::Error),
    Bincode(bincode::Error),
    InputAddress(input_address::Error),
    NotFound,
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn put(
    transaction_a: &TransactionA,
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<(), Error> {
    let key = transaction_a.hash;
    let value = bincode::serialize(&transaction_a.b()).map_err(Error::Bincode)?;
    db.put_cf(crate::transactions(db), key, value)
        .map_err(Error::RocksDB)
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn get_a(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<TransactionA, Error> {
    let input_address = input_address::get(db, hash).ok();
    let transaction_a = get_b(db, hash)?
        .a(input_address)
        .map_err(Error::Transaction)?;
    if input_address.is_none() {
        input_address::put(hash, &transaction_a.input_address, db).map_err(Error::InputAddress)?;
    }
    Ok(transaction_a)
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn get_b(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<TransactionB, Error> {
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
        bincode::serialize(&TransactionB::default()).unwrap().len()
    );
}
