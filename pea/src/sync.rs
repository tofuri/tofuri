use pea_core::constants::BLOCK_TIME_MIN;
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
        self.bps > 1_f32 / BLOCK_TIME_MIN as f32
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
