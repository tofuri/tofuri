use lazy_static::lazy_static;
use serde::Deserialize;
use serde::Serialize;
use tofuri_core::*;
lazy_static! {
    static ref BPS: f32 = 0.5_f32 + (1_f32 / 2_f32.powf(BLOCK_TIME as f32));
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sync {
    pub bps: f32,
    pub new: f32,
    pub completed: bool,
}
impl Sync {
    pub fn handler(&mut self) {
        self.bps += self.new;
        self.bps /= 2.0;
        self.new = 0.0;
        self.completed = !self.downloading();
    }
    pub fn downloading(&self) -> bool {
        self.bps > *BPS
    }
}
