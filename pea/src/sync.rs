#[derive(Debug)]
pub struct Sync {
    pub index: usize,
    pub avg: f32,
    pub new: usize,
    pub downloading: bool,
}
impl Sync {
    pub fn handler(&mut self) {
        self.avg += self.new as f32;
        self.avg /= 2_f32;
        self.new = 0;
        self.downloading = self.avg > 1_f32;
    }
}
impl Default for Sync {
    fn default() -> Self {
        Self {
            index: 0,
            avg: 0.0,
            new: 0,
            downloading: false,
        }
    }
}
