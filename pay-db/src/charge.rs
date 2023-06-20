use crate::Error;
use rocksdb::DB;
use tofuri_key::Key;
use tofuri_pay_core::Charge;
pub fn put(db: &DB, key: &Key, charge: &Charge) -> Result<(), Error> {
    let key = charge.address_bytes(key);
    let value = bincode::serialize(charge).map_err(Error::Bincode)?;
    db.put_cf(super::charges(db), key, value)
        .map_err(Error::RocksDB)
}
pub fn get(db: &DB, hash: &[u8]) -> Result<Charge, Error> {
    let key = hash;
    let vec = db
        .get_cf(super::charges(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
