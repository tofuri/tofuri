use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, DBWithThreadMode, Options, SingleThreaded, DB};
fn descriptors() -> Vec<ColumnFamilyDescriptor> {
    let options = Options::default();
    vec![ColumnFamilyDescriptor::new("charges", options)]
}
pub fn open(path: &str) -> DBWithThreadMode<SingleThreaded> {
    let mut options = Options::default();
    options.create_missing_column_families(true);
    options.create_if_missing(true);
    DB::open_cf_descriptors(&options, path, descriptors()).unwrap()
}
pub fn charges(db: &DBWithThreadMode<SingleThreaded>) -> &ColumnFamily {
    db.cf_handle("charges").unwrap()
}
pub mod charge {
    use pea_key::Key;
    use pea_pay_core::Charge;
    use rocksdb::{DBWithThreadMode, SingleThreaded};
    use std::error::Error;
    pub fn put(db: &DBWithThreadMode<SingleThreaded>, key: &Key, charge: &Charge) -> Result<(), Box<dyn Error>> {
        db.put_cf(super::charges(db), charge.address_bytes(key), bincode::serialize(charge)?)?;
        Ok(())
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<Charge, Box<dyn Error>> {
        Ok(bincode::deserialize(&db.get_cf(super::charges(db), hash)?.ok_or("charge not found")?)?)
    }
}
