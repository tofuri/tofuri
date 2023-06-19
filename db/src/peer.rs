use crate::Error;
use rocksdb::IteratorMode;
use rocksdb::DB;
use std::net::IpAddr;
use tracing::instrument;
#[instrument(skip_all, level = "trace")]
pub fn put(ip_addr: &IpAddr, db: &DB) -> Result<(), Error> {
    let key = bincode::serialize(ip_addr).map_err(Error::Bincode)?;
    let value = [];
    db.put_cf(crate::peers(db), key, value)
        .map_err(Error::RocksDB)
}
#[instrument(skip_all, level = "debug")]
pub fn get_all(db: &DB) -> Result<Vec<IpAddr>, Error> {
    let mut peers: Vec<IpAddr> = vec![];
    for res in db.iterator_cf(crate::peers(db), IteratorMode::Start) {
        let (peer, _) = res.map_err(Error::RocksDB)?;
        peers.push(bincode::deserialize(&peer).map_err(Error::Bincode)?);
    }
    Ok(peers)
}
