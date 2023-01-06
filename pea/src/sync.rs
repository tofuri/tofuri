use lazy_static::lazy_static;
use pea_core::*;
lazy_static! {
    static ref BPS: f32 = 0.5_f32 + (1_f32 / 2_f32.powf(BLOCK_TIME_MIN as f32));
}
#[derive(Debug)]
pub struct Sync {
    pub index: usize,
    pub bps: f32,
    pub new: usize,
    pub completed: bool,
}
impl Sync {
    pub fn handler(&mut self) {
        self.bps += self.new as f32;
        self.bps /= 2_f32;
        self.new = 0;
        self.completed = !self.downloading();
    }
    pub fn downloading(&self) -> bool {
        self.bps > *BPS
    }
}
impl Default for Sync {
    fn default() -> Self {
        Self {
            index: 0,
            bps: 0.0,
            new: 0,
            completed: false,
        }
    }
}
