use crate::input_address;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tofuri_stake::StakeA;
use tofuri_stake::StakeB;
#[derive(Debug)]
pub enum Error {
    Stake(tofuri_stake::Error),
    RocksDB(rocksdb::Error),
    Bincode(bincode::Error),
    InputAddress(input_address::Error),
    NotFound,
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn put(stake_a: &StakeA, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
    let key = stake_a.hash;
    let value = bincode::serialize(&stake_a.b()).map_err(Error::Bincode)?;
    db.put_cf(crate::stakes(db), key, value).map_err(Error::RocksDB)
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn get_a(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<StakeA, Error> {
    let input_address = input_address::get(db, hash).ok();
    let stake_a = get_b(db, hash)?.a(input_address).map_err(Error::Stake)?;
    if input_address.is_none() {
        input_address::put(hash, &stake_a.input_address, db).map_err(Error::InputAddress)?;
    }
    Ok(stake_a)
}
#[tracing::instrument(skip_all, level = "trace")]
pub fn get_b(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<StakeB, Error> {
    let key = hash;
    let vec = db.get_cf(crate::stakes(db), key).map_err(Error::RocksDB)?.ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
#[test]
fn test_serialize_len() {
    assert_eq!(77, bincode::serialize(&StakeB::default()).unwrap().len());
}
