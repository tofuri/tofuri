use crate::beta;
use crate::input_public_key;
use crate::stake;
use crate::transaction;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tofuri_block::Block;
use tofuri_block::BlockB;
use tofuri_block::BlockC;
use tracing::instrument;
#[derive(Debug)]
pub enum Error {
    Block(tofuri_block::Error),
    RocksDB(rocksdb::Error),
    Bincode(bincode::Error),
    Transaction(transaction::Error),
    Stake(stake::Error),
    Beta(beta::Error),
    InputPublicKey(input_public_key::Error),
    NotFound,
}
#[instrument(skip_all, level = "trace")]
pub fn put(block_a: &BlockB, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
    for transaction_a in block_a.transactions.iter() {
        transaction::put(transaction_a, db).map_err(Error::Transaction)?;
    }
    for stake in block_a.stakes.iter() {
        stake::put(stake, db).map_err(Error::Stake)?;
    }
    let key = block_a.hash();
    let value = bincode::serialize(&block_a.c()).map_err(Error::Bincode)?;
    db.put_cf(crate::blocks(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get_b(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<BlockB, Error> {
    let block_c = get_c(db, hash)?;
    let mut transactions = vec![];
    for hash in block_c.transaction_hashes.iter() {
        transactions.push(transaction::get(db, hash).map_err(Error::Transaction)?);
    }
    let mut stakes = vec![];
    for hash in block_c.stake_hashes.iter() {
        stakes.push(stake::get(db, hash).map_err(Error::Stake)?);
    }
    let block_b = block_c.b(transactions, stakes);
    Ok(block_b)
}
#[instrument(skip_all, level = "trace")]
pub fn get_c(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<BlockC, Error> {
    let key = hash;
    let vec = db
        .get_cf(crate::blocks(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
#[test]
fn test_serialize_len() {
    assert_eq!(197, bincode::serialize(&BlockC::default()).unwrap().len());
}
