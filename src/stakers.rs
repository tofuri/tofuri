use crate::{db, types};
use rocksdb::{DBWithThreadMode, SingleThreaded};
use std::{collections::VecDeque, error::Error};
pub fn get(
    db: &DBWithThreadMode<SingleThreaded>,
    hash: &[u8],
) -> Result<VecDeque<(types::PublicKeyBytes, types::Height)>, Box<dyn Error>> {
    Ok(bincode::deserialize(
        &db.get_cf(db::stakers(db), hash)?
            .ok_or("stakers not found")?,
    )?)
}
pub fn put(
    db: &DBWithThreadMode<SingleThreaded>,
    hash: &types::Hash,
    stakers: &VecDeque<(types::PublicKeyBytes, types::Height)>,
) -> Result<(), Box<dyn Error>> {
    db.put_cf(db::stakers(db), hash, bincode::serialize(stakers)?)?;
    Ok(())
}
