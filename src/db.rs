use rocksdb::{
    ColumnFamily, ColumnFamilyDescriptor, DBWithThreadMode, Options, SingleThreaded, DB,
};
use std::error::Error;
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
        ColumnFamilyDescriptor::new("balances", options.clone()),
        ColumnFamilyDescriptor::new("staked_balances", options.clone()),
        ColumnFamilyDescriptor::new("multiaddr", options),
    ]
}
pub fn open(path: &str) -> DBWithThreadMode<SingleThreaded> {
    let mut options = Options::default();
    options.create_missing_column_families(true);
    options.create_if_missing(true);
    DB::open_cf_descriptors(&options, path, get_descriptors()).unwrap()
}
pub fn cf_handle_blocks(
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<&ColumnFamily, Box<dyn Error>> {
    Ok(db
        .cf_handle("blocks")
        .ok_or("blocks column family handle not found")?)
}
pub fn cf_handle_transactions(
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<&ColumnFamily, Box<dyn Error>> {
    Ok(db
        .cf_handle("transactions")
        .ok_or("transactions column family handle not found")?)
}
pub fn cf_handle_stakes(
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<&ColumnFamily, Box<dyn Error>> {
    Ok(db
        .cf_handle("stakes")
        .ok_or("stakes column family handle not found")?)
}
pub fn cf_handle_balances(
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<&ColumnFamily, Box<dyn Error>> {
    Ok(db
        .cf_handle("balances")
        .ok_or("balances column family handle not found")?)
}
pub fn cf_handle_staked_balances(
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<&ColumnFamily, Box<dyn Error>> {
    Ok(db
        .cf_handle("staked_balances")
        .ok_or("staked_balances column family handle not found")?)
}
pub fn cf_handle_multiaddr(
    db: &DBWithThreadMode<SingleThreaded>,
) -> Result<&ColumnFamily, Box<dyn Error>> {
    Ok(db
        .cf_handle("multiaddr")
        .ok_or("multiaddr column family handle not found")?)
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
