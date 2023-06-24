pub mod external;
pub mod internal;
use self::internal::Internal;
use self::internal::Request;
use std::net::IpAddr;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
use tokio::sync::mpsc;
pub enum Call {
    Balance([u8; 20]),
    BalancePendingMin([u8; 20]),
    BalancePendingMax([u8; 20]),
    Staked([u8; 20]),
    StakedPendingMin([u8; 20]),
    StakedPendingMax([u8; 20]),
    Height,
    HeightByHash([u8; 32]),
    BlockLatest,
    HashByHeight(usize),
    BlockByHash([u8; 32]),
    TransactionByHash([u8; 32]),
    StakeByHash([u8; 32]),
    Peers,
    Peer(IpAddr),
    Transaction(Transaction),
    Stake(Stake),
    Address,
    Ticks,
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
pub struct API {
    pub rx: mpsc::Receiver<Request>,
}
impl API {
    pub fn spawn(buffer: usize, api: &str) -> API {
        let (tx, rx) = mpsc::channel(buffer);
        let internal = Internal(tx);
        let api = api.to_string();
        tokio::spawn(async { external::serve(internal, api).await });
        API { rx }
    }
}
