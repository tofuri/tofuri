use std::collections::HashMap;
use std::net::IpAddr;
use tofuri_core::*;
pub enum Endpoint {
    Block,
    Transaction,
    Stake,
    IpAddr,
    SyncRequest,
    SyncResponse,
}
#[derive(Debug, Default)]
pub struct Ratelimit {
    map: HashMap<IpAddr, ([usize; 6], Option<u32>)>,
}
impl Ratelimit {
    pub fn get(&self, ip_addr: &IpAddr) -> ([usize; 6], Option<u32>) {
        *self.map.get(ip_addr).unwrap_or(&([0; 6], None))
    }
    pub fn is_ratelimited(&self, b: &Option<u32>) -> bool {
        if let Some(timestamp) = b {
            if timestamp + RATELIMIT_DURATION > tofuri_util::timestamp() {
                return true;
            }
        }
        false
    }
    pub fn add(&mut self, ip_addr: IpAddr, endpoint: Endpoint) -> bool {
        let mut value = self.get(&ip_addr);
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
            Endpoint::IpAddr => {
                a[3] += 1;
                a[3] > RATELIMIT_IP_ADDR
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
            *b = Some(tofuri_util::timestamp());
        }
        self.map.insert(ip_addr, value);
        ratelimited
    }
    pub fn reset(&mut self) {
        for value in self.map.values_mut() {
            let a = &mut value.0;
            a[0] = a[0].saturating_sub(RATELIMIT_BLOCK);
            a[1] = a[1].saturating_sub(RATELIMIT_TRANSACTION);
            a[2] = a[2].saturating_sub(RATELIMIT_STAKE);
            a[3] = a[3].saturating_sub(RATELIMIT_IP_ADDR);
            a[4] = a[4].saturating_sub(RATELIMIT_SYNC_REQUEST);
            a[5] = a[5].saturating_sub(RATELIMIT_SYNC_RESPONSE);
        }
    }
}
