#[derive(Debug)]
pub struct Sync {
    pub index_0: usize,
    pub index_1: usize,
    pub new: usize,
    pub syncing: bool,
}
impl Sync {
    pub fn handler(&mut self) {
        self.syncing = self.new > 1;
        self.new = 0;
    }
}
impl Default for Sync {
    fn default() -> Self {
        Self {
            index_0: 0,
            index_1: 0,
            new: 0,
            syncing: false,
        }
    }
}
