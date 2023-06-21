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
pub fn open(path: impl AsRef<Path>) -> DB {
    let mut opts = Options::default();
    opts.create_missing_column_families(true);
    opts.create_if_missing(true);
    let options = Options::default();
    let cfs = vec![
        ColumnFamilyDescriptor::new("blocks", options.clone()),
        ColumnFamilyDescriptor::new("transactions", options.clone()),
        ColumnFamilyDescriptor::new("stakes", options.clone()),
        ColumnFamilyDescriptor::new("peers", options.clone()),
        ColumnFamilyDescriptor::new("input addresses", options.clone()),
        ColumnFamilyDescriptor::new("input public keys", options.clone()),
        ColumnFamilyDescriptor::new("betas", options.clone()),
        ColumnFamilyDescriptor::new("checkpoint", options),
    ];
    DB::open_cf_descriptors(&opts, path, cfs).unwrap()
}
