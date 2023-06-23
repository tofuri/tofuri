pub mod internal;
pub mod router;
use axum::Router;
use axum::Server;
use serde::de::DeserializeOwned;
use std::net::IpAddr;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::info;
pub async fn serve(router: Router, api: String) {
    let addr = api.parse().unwrap();
    info!(?addr, "API listening");
    let make_service = router.into_make_service();
    Server::bind(&addr).serve(make_service).await.unwrap();
}
#[derive(Clone)]
pub struct S {
    pub tx: mpsc::Sender<Request>,
}
impl S {
    pub async fn call<T: DeserializeOwned>(&self, call: Call) -> T {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        let _ = self
            .tx
            .send(Request {
                call,
                oneshot_sender,
            })
            .await;
        let vec = oneshot_receiver.await.unwrap();
        bincode::deserialize(&vec).unwrap()
    }
}
pub struct Request {
    pub call: Call,
    pub oneshot_sender: oneshot::Sender<Vec<u8>>,
}
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
