pub mod charge;
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
    let cfs = vec![ColumnFamilyDescriptor::new("charge", options)];
    DB::open_cf_descriptors(&opts, path, cfs).unwrap()
}
