pub mod block;
pub mod checkpoint;
pub mod peer;
pub mod stake;
pub mod transaction;
pub mod tree;
use rocksdb::ColumnFamily;
use rocksdb::ColumnFamilyDescriptor;
use rocksdb::Options;
use rocksdb::DB;
use std::path::Path;
#[derive(Debug)]
pub enum Error {
    RocksDB(rocksdb::Error),
    Bincode(bincode::Error),
    NotFound,
}
pub fn open(path: impl AsRef<Path>) -> DB {
    let mut options = Options::default();
    options.create_missing_column_families(true);
    options.create_if_missing(true);
    DB::open_cf_descriptors(&options, path, descriptors()).unwrap()
}
fn descriptors() -> Vec<ColumnFamilyDescriptor> {
    let options = Options::default();
    vec![
        ColumnFamilyDescriptor::new("blocks", options.clone()),
        ColumnFamilyDescriptor::new("transactions", options.clone()),
        ColumnFamilyDescriptor::new("stakes", options.clone()),
        ColumnFamilyDescriptor::new("peers", options.clone()),
        ColumnFamilyDescriptor::new("input addresses", options.clone()),
        ColumnFamilyDescriptor::new("input public keys", options.clone()),
        ColumnFamilyDescriptor::new("betas", options.clone()),
        ColumnFamilyDescriptor::new("checkpoint", options),
    ]
}
pub fn blocks(db: &DB) -> &ColumnFamily {
    db.cf_handle("blocks").unwrap()
}
pub fn transactions(db: &DB) -> &ColumnFamily {
    db.cf_handle("transactions").unwrap()
}
pub fn stakes(db: &DB) -> &ColumnFamily {
    db.cf_handle("stakes").unwrap()
}
pub fn peers(db: &DB) -> &ColumnFamily {
    db.cf_handle("peers").unwrap()
}
pub fn checkpoint(db: &DB) -> &ColumnFamily {
    db.cf_handle("checkpoint").unwrap()
}
