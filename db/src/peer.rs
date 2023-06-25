use crate::Error;
use rocksdb::ColumnFamily;
use rocksdb::IteratorMode;
use rocksdb::DB;
use std::net::IpAddr;
use tracing::instrument;
pub fn cf(db: &DB) -> &ColumnFamily {
    db.cf_handle("peer").unwrap()
}
#[instrument(skip_all, level = "trace")]
pub fn put(db: &DB, ip_addr: &IpAddr) -> Result<(), Error> {
    let key = bincode::serialize(ip_addr).map_err(Error::Bincode)?;
    let value = [];
    db.put_cf(cf(db), key, value).map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "debug")]
pub fn get_all(db: &DB) -> Result<Vec<IpAddr>, Error> {
    let mut peers: Vec<IpAddr> = vec![];
    for res in db.iterator_cf(cf(db), IteratorMode::Start) {
        let (peer, _) = res.map_err(Error::RocksDB)?;
        peers.push(bincode::deserialize(&peer).map_err(Error::Bincode)?);
    }
    Ok(peers)
}
