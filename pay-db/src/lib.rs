pub mod charge;
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
    vec![ColumnFamilyDescriptor::new("charges", options)]
}
pub fn charges(db: &DB) -> &ColumnFamily {
    db.cf_handle("charges").unwrap()
}
