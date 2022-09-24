use crate::{db, penalty::Penalty, types};
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use std::error::Error;
#[derive(Serialize, Deserialize, Debug)]
pub struct Penalties {
    vec: Vec<Penalty>,
}
impl Penalties {
    pub fn new(vec: Vec<Penalty>) -> Penalties {
        Penalties { vec }
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(Penalties::new(bincode::deserialize(
            &db.get_cf(db::penalties(db), hash)?
                .ok_or("penalties not found")?,
        )?))
    }
    pub fn put(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &types::Hash,
    ) -> Result<(), Box<dyn Error>> {
        db.put_cf(db::penalties(db), hash, bincode::serialize(&self)?)?;
        Ok(())
    }
}
