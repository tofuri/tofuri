use libp2p::PeerId;
use pea_core::RATELIMIT;
use std::collections::HashMap;
#[derive(Debug, Default, Clone, Copy)]
pub struct Score {
    pub new: f32,
    pub avg: f32,
}
#[derive(Debug, Default)]
pub struct Ratelimit {
    map: HashMap<PeerId, Score>,
}
impl Ratelimit {
    pub fn get(&self, peer_id: &PeerId) -> Score {
        match self.map.get(peer_id) {
            Some(x) => *x,
            None => Score::default(),
        }
    }
    pub fn add(&mut self, peer_id: PeerId) -> bool {
        let mut score = self.get(&peer_id);
        score.new += 1.0;
        self.map.insert(peer_id, score);
        if score.new >= RATELIMIT {
            return true;
        }
        if score.avg >= RATELIMIT {
            return true;
        }
        false
    }
    pub fn update(&mut self) {
        for score in self.map.values_mut() {
            score.avg += score.new;
            score.avg /= 2.0;
            score.new = 0.0;
        }
    }
}
