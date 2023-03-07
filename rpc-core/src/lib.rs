use serde::Deserialize;
use serde::Serialize;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Type {
    Balance,
    BalancePendingMin,
    BalancePendingMax,
    Staked,
    StakedPendingMin,
    StakedPendingMax,
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
    Time,
    TreeSize,
    Sync,
    RandomQueue,
    UnstableHashes,
    UnstableLatestHashes,
    UnstableStakers,
    StableHashes,
    StableLatestHashes,
    StableStakers,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    pub t: Type,
    pub v: Vec<u8>,
}
