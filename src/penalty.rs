use crate::{address, types};
use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Serialize, Deserialize)]
pub struct Penalty {
    public_key: types::PublicKeyBytes,
    balance_staked: types::Amount,
}
impl Penalty {
    pub fn new(public_key: types::PublicKeyBytes, balance_staked: types::Amount) -> Penalty {
        Penalty {
            public_key,
            balance_staked,
        }
    }
}
impl fmt::Debug for Penalty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #![allow(dead_code)]
        #[derive(Debug)]
        struct Penalty {
            public_key: String,
            balance_staked: types::Amount,
        }
        write!(
            f,
            "{:?}",
            Penalty {
                public_key: address::encode(&self.public_key),
                balance_staked: self.balance_staked
            }
        )
    }
}
