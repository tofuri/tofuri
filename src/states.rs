use crate::state::State;
#[derive(Debug)]
pub struct States {
    current: State,
    previous: State,
}
impl States {
    pub fn new() -> States {
        States {
            current: State::new(),
            previous: State::new(),
        }
    }
    pub fn get_current(&self) -> &State {
        &self.current
    }
    pub fn get_current_mut(&mut self) -> &mut State {
        &mut self.current
    }
    pub fn get_previous(&self) -> &State {
        &self.previous
    }
    pub fn get_previous_mut(&mut self) -> &mut State {
        &mut self.previous
    }
}
