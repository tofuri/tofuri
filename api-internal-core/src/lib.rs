use serde::Deserialize;
use serde::Serialize;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Data {
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
    CargoPkgName,
    CargoPkgVersion,
    CargoPkgRepository,
    GitHash,
    Address,
    Ticks,
    Tps,
    Lag,
    Time,
    TreeSize,
    Sync,
    RandomQueue,
    DynamicHashes,
    DynamicLatestHashes,
    DynamicStakers,
    TrustedHashes,
    TrustedLatestHashes,
    TrustedStakers,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    pub data: Data,
    pub vec: Vec<u8>,
}
