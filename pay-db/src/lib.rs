use rocksdb::ColumnFamily;
use rocksdb::ColumnFamilyDescriptor;
use rocksdb::DBWithThreadMode;
use rocksdb::Options;
use rocksdb::SingleThreaded;
use rocksdb::DB;
use std::path::Path;
fn descriptors() -> Vec<ColumnFamilyDescriptor> {
    let options = Options::default();
    vec![ColumnFamilyDescriptor::new("charges", options)]
}
pub fn open(path: impl AsRef<Path>) -> DBWithThreadMode<SingleThreaded> {
    let mut options = Options::default();
    options.create_missing_column_families(true);
    options.create_if_missing(true);
    DB::open_cf_descriptors(&options, path, descriptors()).unwrap()
}
pub fn charges(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("charges").unwrap()
}
pub mod charge {
    use rocksdb::DBWithThreadMode;
    use rocksdb::SingleThreaded;
    use std::error::Error;
    use tofuri_key::Key;
    use tofuri_pay_core::Charge;
    pub fn put(db: &DBWithThreadMode<SingleThreaded>, key: &Key, charge: &Charge) -> Result<(), Box<dyn Error>> {
        db.put_cf(super::charges(db), charge.address_bytes(key), bincode::serialize(charge)?)?;
        Ok(())
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<Charge, Box<dyn Error>> {
        Ok(bincode::deserialize(&db.get_cf(super::charges(db), hash)?.ok_or("charge not found")?)?)
    }
}
