use crate::constants::BLOCK_TIME_MIN;
#[derive(Debug)]
pub struct Sync {
    pub index: usize,
    pub new: usize,
    history: [usize; BLOCK_TIME_MIN],
    pub syncing: bool,
}
impl Sync {
    pub fn handler(&mut self) {
        self.history.rotate_right(1);
        self.history[0] = self.new;
        self.new = 0;
        let mut sum = 0;
        for x in self.history {
            sum += x;
        }
        self.syncing = sum > 1;
    }
}
impl Default for Sync {
    fn default() -> Self {
        Self {
            index: 0,
            new: 0,
            history: [0; BLOCK_TIME_MIN],
            syncing: true,
        }
    }
}
