use crate::block::BlockDB;
use crate::Error;
use rocksdb::IteratorMode;
use rocksdb::DB;
use std::collections::HashMap;
use tofuri_tree::Tree;
use tofuri_tree::GENESIS_BLOCK_PREVIOUS_HASH;
use tracing::instrument;
#[instrument(skip_all, level = "debug")]
pub fn reload(tree: &mut Tree, db: &DB) -> Result<(), Error> {
    tree.clear();
    let mut map: HashMap<[u8; 32], Vec<([u8; 32], u32)>> = HashMap::new();
    for res in db.iterator_cf(crate::blocks(db), IteratorMode::Start) {
        let (key, value) = res.map_err(Error::RocksDB)?;
        let hash = bincode::deserialize(&key).map_err(Error::Bincode)?;
        let block_metadata: BlockDB = bincode::deserialize(&value).map_err(Error::Bincode)?;
        match map.get(&block_metadata.previous_hash) {
            Some(vec) => {
                let mut vec = vec.clone();
                vec.push((hash, block_metadata.timestamp));
                map.insert(block_metadata.previous_hash, vec);
            }
            None => {
                map.insert(
                    block_metadata.previous_hash,
                    vec![(hash, block_metadata.timestamp)],
                );
            }
        };
    }
    if map.is_empty() {
        return Ok(());
    }
    let previous_hash = GENESIS_BLOCK_PREVIOUS_HASH;
    let mut previous_hashes = vec![previous_hash];
    let mut hashes_0 = vec![];
    for (hash, timestamp) in map.get(&previous_hash).expect("genesis block hashes") {
        hashes_0.push((*hash, *timestamp));
    }
    let mut vec = vec![];
    loop {
        let mut hashes_1 = vec![];
        for previous_hash in previous_hashes.clone() {
            for (hash, timestamp) in hashes_0.clone() {
                vec.push((hash, previous_hash, timestamp));
                if let Some(vec) = map.remove(&hash) {
                    for (hash, timestamp) in vec {
                        hashes_1.push((hash, timestamp));
                    }
                };
            }
        }
        if hashes_1.is_empty() {
            break;
        }
        previous_hashes.clear();
        for (hash, _) in hashes_0 {
            previous_hashes.push(hash);
        }
        hashes_0 = hashes_1;
    }
    for (hash, previous_hash, timestamp) in vec {
        tree.insert(hash, previous_hash, timestamp);
    }
    tree.sort_branches();
    Ok(())
}
