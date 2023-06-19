use crate::stake;
use crate::transaction;
use crate::Error;
use rocksdb::DB;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use tofuri_block::Block;
use tracing::instrument;
#[instrument(skip_all, level = "trace")]
pub fn put(block: &Block, db: &DB) -> Result<(), Error> {
    for transaction_a in block.transactions.iter() {
        transaction::put(transaction_a, db)?;
    }
    for stake in block.stakes.iter() {
        stake::put(stake, db)?;
    }
    let key = block.hash();
    let value = bincode::serialize(&BlockDB::from(block)).map_err(Error::Bincode)?;
    db.put_cf(crate::blocks(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DB, hash: &[u8]) -> Result<Block, Error> {
    let key = hash;
    let vec = db
        .get_cf(crate::blocks(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    let block_db: BlockDB = bincode::deserialize(&vec).map_err(Error::Bincode)?;
    let mut transactions = vec![];
    for hash in block_db.transaction_hashes.iter() {
        transactions.push(transaction::get(db, hash)?);
    }
    let mut stakes = vec![];
    for hash in block_db.stake_hashes.iter() {
        stakes.push(stake::get(db, hash)?);
    }
    Ok(Block {
        previous_hash: block_db.previous_hash,
        timestamp: block_db.timestamp,
        signature: block_db.signature,
        pi: block_db.pi,
        transactions,
        stakes,
    })
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockDB {
    pub previous_hash: [u8; 32],
    pub timestamp: u32,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    #[serde(with = "BigArray")]
    pub pi: [u8; 81],
    pub transaction_hashes: Vec<[u8; 32]>,
    pub stake_hashes: Vec<[u8; 32]>,
}
impl From<&Block> for BlockDB {
    fn from(block: &Block) -> BlockDB {
        BlockDB {
            previous_hash: block.previous_hash,
            timestamp: block.timestamp,
            signature: block.signature,
            pi: block.pi,
            transaction_hashes: block.transaction_hashes(),
            stake_hashes: block.stake_hashes(),
        }
    }
}
impl Default for BlockDB {
    fn default() -> BlockDB {
        BlockDB {
            previous_hash: [0; 32],
            timestamp: 0,
            signature: [0; 64],
            pi: [0; 81],
            transaction_hashes: vec![],
            stake_hashes: vec![],
        }
    }
}
