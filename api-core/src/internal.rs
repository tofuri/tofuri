use serde::Deserialize;
use serde::Serialize;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Data {
    Info,
    Sync,
    Dynamic,
    Trusted,
    Args,
    Balance,
    Staked,
    Height,
    HeightByHash,
    BlockLatest,
    HashByHeight,
    BlockByHash,
    TransactionByHash,
    StakeByHash,
    Peers,
    Peer,
    Transaction,
    Stake,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    pub data: Data,
    pub vec: Vec<u8>,
}
