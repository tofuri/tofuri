use crate::types;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct Sync {
    pub height: types::Height,
}
impl Sync {
    pub fn new(height: types::Height) -> Sync {
        Sync { height }
    }
}
