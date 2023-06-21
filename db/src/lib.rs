pub mod block;
pub mod checkpoint;
pub mod peer;
pub mod stake;
pub mod transaction;
pub mod tree;
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
pub fn open_cf_descriptors(path: impl AsRef<Path>) -> DB {
    let mut opts = Options::default();
    opts.create_missing_column_families(true);
    opts.create_if_missing(true);
    let options = Options::default();
    let cfs = vec![
        ColumnFamilyDescriptor::new("block", options.clone()),
        ColumnFamilyDescriptor::new("transaction", options.clone()),
        ColumnFamilyDescriptor::new("stake", options.clone()),
        ColumnFamilyDescriptor::new("peer", options.clone()),
        ColumnFamilyDescriptor::new("checkpoint", options),
    ];
    DB::open_cf_descriptors(&opts, path, cfs).unwrap()
}
