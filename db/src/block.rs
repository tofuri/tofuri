use crate::beta;
use crate::input_public_key;
use crate::stake;
use crate::transaction;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use serde::Deserialize;
use serde::Serialize;
use serde_big_array::BigArray;
use tofuri_block::Block;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
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
pub fn put(block: &Block, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Error> {
    for transaction_a in block.transactions.iter() {
        transaction::put(transaction_a, db).map_err(Error::Transaction)?;
    }
    for stake in block.stakes.iter() {
        stake::put(stake, db).map_err(Error::Stake)?;
    }
    let key = block.hash();
    let value = bincode::serialize(&BlockDB::from(block)).map_err(Error::Bincode)?;
    db.put_cf(crate::blocks(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<Block, Error> {
    let key = hash;
    let vec = db
        .get_cf(crate::blocks(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    let block_db: BlockDB = bincode::deserialize(&vec).map_err(Error::Bincode)?;
    let mut transactions = vec![];
    for hash in block_db.transaction_hashes.iter() {
        transactions.push(transaction::get(db, hash).map_err(Error::Transaction)?);
    }
    let mut stakes = vec![];
    for hash in block_db.stake_hashes.iter() {
        stakes.push(stake::get(db, hash).map_err(Error::Stake)?);
    }
    let block_b = block_db.block(transactions, stakes);
    Ok(block_b)
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
impl BlockDB {
    pub fn block(&self, transactions: Vec<Transaction>, stakes: Vec<Stake>) -> Block {
        Block {
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            signature: self.signature,
            pi: self.pi,
            transactions,
            stakes,
        }
    }
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
#[test]
fn test_serialize_len() {
    assert_eq!(197, bincode::serialize(&BlockDB::default()).unwrap().len());
}
