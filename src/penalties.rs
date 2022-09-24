use crate::{db, penalty::Penalty, types};
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use std::error::Error;
#[derive(Serialize, Deserialize, Debug)]
pub struct Penalties {
    vec: Vec<Penalty>,
}
impl Penalties {
    pub fn new() -> Penalties {
        Penalties { vec: vec![] }
    }
    pub fn get(db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) -> Self {
        let mut penalties = Self::default();
        penalties.set_vec(
            bincode::deserialize(
                &db.get_cf(db::penalties(db), hash)
                    .unwrap()
                    .ok_or("penalties not found")
                    .unwrap(),
            )
            .unwrap(),
        );
        penalties
    }
    fn set_vec(&mut self, vec: Vec<Penalty>) {
        self.vec = vec;
    }
    pub fn load(&mut self, db: &DBWithThreadMode<SingleThreaded>, hash: &[u8]) {
        self.vec = bincode::deserialize(
            &db.get_cf(db::penalties(db), hash)
                .unwrap()
                .ok_or("penalties not found")
                .unwrap(),
        )
        .unwrap();
    }
    pub fn put(
        &self,
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &types::Hash,
    ) -> Result<(), Box<dyn Error>> {
        db.put_cf(db::penalties(db), hash, bincode::serialize(&self)?)?;
        Ok(())
    }
    pub fn get_vec(&self) -> &Vec<Penalty> {
        &self.vec
    }
    pub fn push(&mut self, penalty: Penalty) {
        self.vec.push(penalty)
    }
    pub fn clear(&mut self) {
        self.vec.clear();
    }
}
impl Default for Penalties {
    fn default() -> Self {
        Self::new()
    }
}
