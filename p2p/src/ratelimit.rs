use crate::multiaddr;
use crate::P2p;
use libp2p::PeerId;
use pea_core::*;
use std::collections::HashMap;
use std::error::Error;
use std::net::IpAddr;
pub enum Endpoint {
    Block,
    Transaction,
    Stake,
    Multiaddr,
    SyncRequest,
    SyncResponse,
}
#[derive(Debug, Default)]
pub struct Ratelimit {
    map: HashMap<IpAddr, ([usize; 6], Option<u32>)>,
}
impl Ratelimit {
    pub fn get(&self, addr: &IpAddr) -> ([usize; 6], Option<u32>) {
        *self.map.get(addr).unwrap_or(&([0; 6], None))
    }
    pub fn is_ratelimited(&self, b: &Option<u32>) -> bool {
        if let Some(timestamp) = b {
            if timestamp + RATELIMIT_DURATION > pea_util::timestamp() {
                return true;
            }
        }
        false
    }
    pub fn add(&mut self, addr: IpAddr, endpoint: Endpoint) -> bool {
        let mut value = self.get(&addr);
        let a = &mut value.0;
        let b = &mut value.1;
        if self.is_ratelimited(b) {
            return true;
        }
        let ratelimited = match endpoint {
            Endpoint::Block => {
                a[0] += 1;
                a[0] > RATELIMIT_BLOCK
            }
            Endpoint::Transaction => {
                a[1] += 1;
                a[1] > RATELIMIT_TRANSACTION
            }
            Endpoint::Stake => {
                a[2] += 1;
                a[2] > RATELIMIT_STAKE
            }
            Endpoint::Multiaddr => {
                a[3] += 1;
                a[3] > RATELIMIT_MULTIADDR
            }
            Endpoint::SyncRequest => {
                a[4] += 1;
                a[4] > RATELIMIT_SYNC_REQUEST
            }
            Endpoint::SyncResponse => {
                a[5] += 1;
                a[5] > RATELIMIT_SYNC_RESPONSE
            }
        };
        if ratelimited {
            *b = Some(pea_util::timestamp());
        }
        self.map.insert(addr, value);
        ratelimited
    }
    pub fn reset(&mut self) {
        for value in self.map.values_mut() {
            let a = &mut value.0;
            a[0] = a[0].saturating_sub(RATELIMIT_BLOCK);
            a[1] = a[1].saturating_sub(RATELIMIT_TRANSACTION);
            a[2] = a[2].saturating_sub(RATELIMIT_STAKE);
            a[3] = a[3].saturating_sub(RATELIMIT_MULTIADDR);
            a[4] = a[4].saturating_sub(RATELIMIT_SYNC_REQUEST);
            a[5] = a[5].saturating_sub(RATELIMIT_SYNC_RESPONSE);
        }
    }
    pub fn ratelimit(p2p: &mut P2p, peer_id: PeerId, endpoint: Endpoint) -> Result<(), Box<dyn Error>> {
        let (multiaddr, _) = p2p.connections.iter().find(|x| x.1 == &peer_id).unwrap();
        let addr = multiaddr::multiaddr_addr(multiaddr).expect("multiaddr to include ip");
        if p2p.ratelimit.add(addr, endpoint) {
            let _ = p2p.swarm.disconnect_peer_id(peer_id);
            return Err("ratelimited".into());
        }
        Ok(())
    }
}
