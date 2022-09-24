use rocksdb::{
    ColumnFamily, ColumnFamilyDescriptor, DBWithThreadMode, Options, SingleThreaded, DB,
};
pub enum Key {
    LatestBlockHash,
}
pub fn key(key: &Key) -> &[u8] {
    match *key {
        Key::LatestBlockHash => &[0],
    }
}
fn get_descriptors() -> Vec<ColumnFamilyDescriptor> {
    let mut options = Options::default();
    options.set_max_write_buffer_number(16);
    vec![
        ColumnFamilyDescriptor::new("blocks", options.clone()),
        ColumnFamilyDescriptor::new("transactions", options.clone()),
        ColumnFamilyDescriptor::new("stakes", options.clone()),
        ColumnFamilyDescriptor::new("stakers", options.clone()),
        ColumnFamilyDescriptor::new("penalties", options),
    ]
}
pub fn open(path: &str) -> DBWithThreadMode<SingleThreaded> {
    let mut options = Options::default();
    options.create_missing_column_families(true);
    options.create_if_missing(true);
    DB::open_cf_descriptors(&options, path, get_descriptors()).unwrap()
}
pub fn blocks(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("blocks").unwrap()
}
pub fn transactions(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("transactions").unwrap()
}
pub fn stakes(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("stakes").unwrap()
}
pub fn stakers(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("stakers").unwrap()
}
pub fn penalties(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("penalties").unwrap()
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;
    use test::Bencher;
    #[bench]
    fn bench_put(b: &mut Bencher) {
        let tempdir = TempDir::new("rocksdb").unwrap();
        let db = open(tempdir.path().to_str().unwrap());
        b.iter(|| db.put(b"test", b"value"));
    }
    #[bench]
    fn bench_get(b: &mut Bencher) {
        let tempdir = TempDir::new("rocksdb").unwrap();
        let db = open(tempdir.path().to_str().unwrap());
        b.iter(|| db.get(b"test"));
    }
}
