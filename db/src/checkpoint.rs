use crate::Error;
use rocksdb::DB;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::VecDeque;
use tofuri_block::Block;
use tracing::instrument;
#[instrument(skip_all, level = "trace")]
pub fn put(db: &DB, checkpoint: &CheckpointDB) -> Result<(), Error> {
    let key = [];
    let value = bincode::serialize(checkpoint).map_err(Error::Bincode)?;
    db.put_cf(crate::checkpoint(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "trace")]
pub fn get(db: &DB) -> Result<CheckpointDB, Error> {
    let key = [];
    let vec = db
        .get_cf(crate::checkpoint(db), key)
        .map_err(Error::RocksDB)?
        .ok_or(Error::NotFound)?;
    bincode::deserialize(&vec).map_err(Error::Bincode)
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CheckpointDB {
    pub height: usize,
    pub latest_block: Block,
    pub stakers: VecDeque<[u8; 20]>,
    pub latest_blocks: Vec<Block>,
    pub map_balance: HashMap<[u8; 20], u128>,
    pub map_staked: HashMap<[u8; 20], u128>,
}
