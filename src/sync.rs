use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct Sync {
    pub height: usize,
}
impl Sync {
    pub fn new(height: usize) -> Sync {
        Sync { height }
    }
}
