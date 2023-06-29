use crate::Node;
use crate::CARGO_PKG_NAME;
use crate::CARGO_PKG_REPOSITORY;
use crate::CARGO_PKG_VERSION;
use crate::GIT_HASH;
use address::public;
use api::BlockHex;
use api::Root;
use api::StakeHex;
use api::TransactionHex;
use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use axum::Server;
use block::Block;
use chrono::offset::Utc;
use fork::BLOCK_TIME;
use hex;
use serde::de::DeserializeOwned;
use stake::Stake;
use std::convert::TryInto;
use std::net::IpAddr;
use std::net::SocketAddr;
use sync::Sync;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::error;
use transaction::Transaction;
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
pub struct APIServer {
    pub rx: mpsc::Receiver<Request>,
}
#[derive(Clone)]
pub struct APIClient {
    pub tx: mpsc::Sender<Request>,
}
impl APIClient {
    pub async fn call<T: DeserializeOwned>(&self, call: Call) -> T {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(Request { call, tx }).await;
        let response = rx.await.unwrap();
        bincode::deserialize(&response.0).unwrap()
    }
}
pub struct Request {
    pub call: Call,
    pub tx: oneshot::Sender<Response>,
}
pub struct Response(pub Vec<u8>);
pub fn channel(buffer: usize) -> (APIClient, APIServer) {
    let (tx, rx) = mpsc::channel(buffer);
    (APIClient { tx }, APIServer { rx })
}
pub fn spawn(api_client: APIClient, addr: &SocketAddr) {
    let builder = Server::bind(addr);
    let router = Router::new()
        .route("/", get(e::root))
        .route("/balance/:address", get(e::balance))
        .route("/balance_pending_min/:address", get(e::balance_pending_min))
        .route("/balance_pending_max/:address", get(e::balance_pending_max))
        .route("/staked/:address", get(e::staked))
        .route("/staked_pending_min/:address", get(e::staked_pending_min))
        .route("/staked_pending_max/:address", get(e::staked_pending_max))
        .route("/height", get(e::height))
        .route("/height/:hash", get(e::height_by_hash))
        .route("/block", get(e::block_latest))
        .route("/hash/:height", get(e::hash_by_height))
        .route("/block/:hash", get(e::block_by_hash))
        .route("/transaction/:hash", get(e::transaction_by_hash))
        .route("/stake/:hash", get(e::stake_by_hash))
        .route("/peers", get(e::peers))
        .route("/peer/:ip_addr", get(e::peer))
        .route("/transaction", post(e::transaction))
        .route("/stake", post(e::stake))
        .route("/cargo_pkg_name", get(e::cargo_pkg_name))
        .route("/cargo_pkg_version", get(e::cargo_pkg_version))
        .route("/cargo_pkg_repository", get(e::cargo_pkg_repository))
        .route("/git_hash", get(e::git_hash))
        .route("/address", get(e::address))
        .route("/ticks", get(e::ticks))
        .route("/time", get(e::time))
        .route("/tree_size", get(e::tree_size))
        .route("/sync", get(e::sync))
        .route("/random_queue", get(e::random_queue))
        .route("/unstable_hashes", get(e::unstable_hashes))
        .route("/unstable_latest_hashes", get(e::unstable_latest_hashes))
        .route("/unstable_stakers", get(e::unstable_stakers))
        .route("/stable_hashes", get(e::stable_hashes))
        .route("/stable_latest_hashes", get(e::stable_latest_hashes))
        .route("/stable_stakers", get(e::stable_stakers))
        .route("/sync_remaining", get(e::sync_remaining))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(api_client);
    let make_service = router.into_make_service();
    tokio::spawn(async { builder.serve(make_service).await });
}
pub async fn accept(node: &mut Node, request: Request) {
    let res = match request.call {
        Call::Balance(a) => i::balance(node, a),
        Call::BalancePendingMin(a) => i::balance_pending_min(node, a),
        Call::BalancePendingMax(a) => i::balance_pending_max(node, a),
        Call::Staked(a) => i::staked(node, a),
        Call::StakedPendingMin(a) => i::staked_pending_min(node, a),
        Call::StakedPendingMax(a) => i::staked_pending_max(node, a),
        Call::Height => i::height(node),
        Call::HeightByHash(a) => i::height_by_hash(node, a),
        Call::BlockLatest => i::block_latest(node),
        Call::HashByHeight(a) => i::hash_by_height(node, a),
        Call::BlockByHash(a) => i::block_by_hash(node, a),
        Call::TransactionByHash(a) => i::transaction_by_hash(node, a),
        Call::StakeByHash(a) => i::stake_by_hash(node, a),
        Call::Peers => i::peers(node),
        Call::Peer(a) => i::peer(node, a),
        Call::Transaction(a) => i::transaction(node, a),
        Call::Stake(a) => i::stake(node, a),
        Call::Address => i::address(node),
        Call::Ticks => i::ticks(node),
        Call::TreeSize => i::tree_size(node),
        Call::Sync => i::sync(node),
        Call::RandomQueue => i::random_queue(node),
        Call::UnstableHashes => i::unstable_hashes(node),
        Call::UnstableLatestHashes => i::unstable_latest_hashes(node),
        Call::UnstableStakers => i::unstable_stakers(node),
        Call::StableHashes => i::stable_hashes(node),
        Call::StableLatestHashes => i::stable_latest_hashes(node),
        Call::StableStakers => i::stable_stakers(node),
    };
    match res {
        Ok(vec) => {
            let _ = request.tx.send(Response(vec));
        }
        Err(e) => {
            error!(?e);
        }
    };
}
pub mod e {
    use super::*;
    pub async fn root() -> impl IntoResponse {
        Json(Root {
            cargo_pkg_name: CARGO_PKG_NAME.to_string(),
            cargo_pkg_version: CARGO_PKG_VERSION.to_string(),
            cargo_pkg_repository: CARGO_PKG_REPOSITORY.to_string(),
            git_hash: GIT_HASH.to_string(),
        })
    }
    pub async fn cargo_pkg_name() -> impl IntoResponse {
        Json(CARGO_PKG_NAME)
    }
    pub async fn cargo_pkg_version() -> impl IntoResponse {
        Json(CARGO_PKG_VERSION)
    }
    pub async fn cargo_pkg_repository() -> impl IntoResponse {
        Json(CARGO_PKG_REPOSITORY)
    }
    pub async fn git_hash() -> impl IntoResponse {
        Json(GIT_HASH)
    }
    pub async fn balance(State(c): State<APIClient>, address: Path<String>) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(c.call::<u128>(Call::Balance(address_bytes)).await)
    }
    pub async fn balance_pending_min(
        State(c): State<APIClient>,
        address: Path<String>,
    ) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(c.call::<u128>(Call::BalancePendingMin(address_bytes)).await)
    }
    pub async fn balance_pending_max(
        State(c): State<APIClient>,
        address: Path<String>,
    ) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(c.call::<u128>(Call::BalancePendingMax(address_bytes)).await)
    }
    pub async fn staked(State(c): State<APIClient>, address: Path<String>) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(c.call::<u128>(Call::Staked(address_bytes)).await)
    }
    pub async fn staked_pending_min(
        State(c): State<APIClient>,
        address: Path<String>,
    ) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(c.call::<u128>(Call::StakedPendingMin(address_bytes)).await)
    }
    pub async fn staked_pending_max(
        State(c): State<APIClient>,
        address: Path<String>,
    ) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(c.call::<u128>(Call::StakedPendingMax(address_bytes)).await)
    }
    pub async fn height(State(c): State<APIClient>) -> impl IntoResponse {
        Json(c.call::<usize>(Call::Height).await)
    }
    pub async fn height_by_hash(
        State(c): State<APIClient>,
        hash: Path<String>,
    ) -> impl IntoResponse {
        let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
        Json(c.call::<usize>(Call::HeightByHash(hash)).await)
    }
    pub async fn block_latest(State(c): State<APIClient>) -> impl IntoResponse {
        let block = c.call::<Block>(Call::BlockLatest).await;
        let block_hex: BlockHex = block.try_into().unwrap();
        Json(block_hex)
    }
    pub async fn hash_by_height(
        State(c): State<APIClient>,
        height: Path<String>,
    ) -> impl IntoResponse {
        let height: usize = height.parse().unwrap();
        let hash = c.call::<[u8; 32]>(Call::HashByHeight(height)).await;
        let hash_hex = hex::encode(hash);
        Json(hash_hex)
    }
    pub async fn block_by_hash(
        State(c): State<APIClient>,
        hash: Path<String>,
    ) -> impl IntoResponse {
        let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
        let block = c.call::<Block>(Call::BlockByHash(hash)).await;
        let block_hex: BlockHex = block.try_into().unwrap();
        Json(block_hex)
    }
    pub async fn transaction_by_hash(
        State(c): State<APIClient>,
        hash: Path<String>,
    ) -> impl IntoResponse {
        let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
        let transaction = c.call::<Transaction>(Call::TransactionByHash(hash)).await;
        let transaction_hex: TransactionHex = transaction.try_into().unwrap();
        Json(transaction_hex)
    }
    pub async fn stake_by_hash(
        State(c): State<APIClient>,
        hash: Path<String>,
    ) -> impl IntoResponse {
        let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
        let stake = c.call::<Stake>(Call::StakeByHash(hash)).await;
        let stake_hex: StakeHex = stake.try_into().unwrap();
        Json(stake_hex)
    }
    pub async fn peers(State(c): State<APIClient>) -> impl IntoResponse {
        Json(c.call::<Vec<IpAddr>>(Call::Peers).await)
    }
    pub async fn peer(
        State(c): State<APIClient>,
        Path(ip_addr): Path<String>,
    ) -> impl IntoResponse {
        let ip_addr = ip_addr.parse().unwrap();
        Json(c.call::<bool>(Call::Peer(ip_addr)).await)
    }
    pub async fn transaction(
        State(c): State<APIClient>,
        Json(transaction): Json<TransactionHex>,
    ) -> impl IntoResponse {
        let transaction: Transaction = transaction.try_into().unwrap();
        Json(c.call::<bool>(Call::Transaction(transaction)).await)
    }
    pub async fn stake(
        State(c): State<APIClient>,
        Json(stake): Json<StakeHex>,
    ) -> impl IntoResponse {
        let stake: Stake = stake.try_into().unwrap();
        Json(c.call::<bool>(Call::Stake(stake)).await)
    }
    pub async fn address(State(c): State<APIClient>) -> impl IntoResponse {
        Json(public::encode(&c.call::<[u8; 20]>(Call::Address).await))
    }
    pub async fn ticks(State(c): State<APIClient>) -> impl IntoResponse {
        Json(c.call::<usize>(Call::Ticks).await)
    }
    pub async fn time() -> impl IntoResponse {
        Json(chrono::offset::Utc::now().timestamp_millis())
    }
    pub async fn tree_size(State(c): State<APIClient>) -> impl IntoResponse {
        Json(c.call::<usize>(Call::TreeSize).await)
    }
    pub async fn sync(State(c): State<APIClient>) -> impl IntoResponse {
        let sync = c.call::<Sync>(Call::Sync).await;
        Json(sync)
    }
    pub async fn random_queue(State(c): State<APIClient>) -> impl IntoResponse {
        Json(
            c.call::<Vec<[u8; 20]>>(Call::RandomQueue)
                .await
                .iter()
                .map(public::encode)
                .collect::<Vec<_>>(),
        )
    }
    pub async fn unstable_hashes(State(c): State<APIClient>) -> impl IntoResponse {
        Json(c.call::<usize>(Call::UnstableHashes).await)
    }
    pub async fn unstable_latest_hashes(State(c): State<APIClient>) -> impl IntoResponse {
        Json(
            c.call::<Vec<[u8; 32]>>(Call::UnstableLatestHashes)
                .await
                .iter()
                .map(hex::encode)
                .collect::<Vec<_>>(),
        )
    }
    pub async fn unstable_stakers(State(c): State<APIClient>) -> impl IntoResponse {
        Json(c.call::<usize>(Call::UnstableStakers).await)
    }
    pub async fn stable_hashes(State(c): State<APIClient>) -> impl IntoResponse {
        Json(c.call::<usize>(Call::StableHashes).await)
    }
    pub async fn stable_latest_hashes(State(c): State<APIClient>) -> impl IntoResponse {
        Json(
            c.call::<Vec<[u8; 32]>>(Call::StableLatestHashes)
                .await
                .iter()
                .map(hex::encode)
                .collect::<Vec<_>>(),
        )
    }
    pub async fn stable_stakers(State(c): State<APIClient>) -> impl IntoResponse {
        Json(c.call::<usize>(Call::StableStakers).await)
    }
    pub async fn sync_remaining(State(c): State<APIClient>) -> impl IntoResponse {
        let sync = c.call::<Sync>(Call::Sync).await;
        if sync.completed {
            return Json(0.0);
        }
        if !sync.downloading() {
            return Json(-1.0);
        }
        let block = c.call::<Block>(Call::BlockLatest).await;
        let mut diff = (Utc::now().timestamp() as u32).saturating_sub(block.timestamp) as f32;
        diff /= BLOCK_TIME as f32;
        diff /= sync.bps;
        Json(diff)
    }
}
pub mod i {
    use super::*;
    #[derive(Debug)]
    pub enum Error {
        Blockchain(blockchain::Error),
        DB(db::Error),
        Bincode(bincode::Error),
    }
    pub fn balance(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.balance(&address)).map_err(Error::Bincode)
    }
    pub fn balance_pending_min(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.balance_pending_min(&address)).map_err(Error::Bincode)
    }
    pub fn balance_pending_max(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.balance_pending_max(&address)).map_err(Error::Bincode)
    }
    pub fn staked(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.staked(&address)).map_err(Error::Bincode)
    }
    pub fn staked_pending_min(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.staked_pending_min(&address)).map_err(Error::Bincode)
    }
    pub fn staked_pending_max(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.staked_pending_max(&address)).map_err(Error::Bincode)
    }
    pub fn height(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.height()).map_err(Error::Bincode)
    }
    pub fn height_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
        bincode::serialize(
            &node
                .blockchain
                .height_by_hash(&hash)
                .map_err(Error::Blockchain)?,
        )
        .map_err(Error::Bincode)
    }
    pub fn block_latest(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.forks.unstable.latest_block).map_err(Error::Bincode)
    }
    pub fn hash_by_height(node: &mut Node, height: usize) -> Result<Vec<u8>, Error> {
        bincode::serialize(
            &node
                .blockchain
                .hash_by_height(height)
                .map_err(Error::Blockchain)?,
        )
        .map_err(Error::Bincode)
    }
    pub fn block_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&db::block::get(&node.db, &hash).map_err(Error::DB)?)
            .map_err(Error::Bincode)
    }
    pub fn transaction_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&db::transaction::get(&node.db, &hash).map_err(Error::DB)?)
            .map_err(Error::Bincode)
    }
    pub fn stake_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&db::stake::get(&node.db, &hash).map_err(Error::DB)?)
            .map_err(Error::Bincode)
    }
    pub fn peers(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.p2p.connections.values().collect::<Vec<_>>())
            .map_err(Error::Bincode)
    }
    pub fn peer(node: &mut Node, ip_addr: IpAddr) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.p2p.connections_unknown.insert(ip_addr)).map_err(Error::Bincode)
    }
    pub fn transaction(node: &mut Node, transaction: Transaction) -> Result<Vec<u8>, Error> {
        bincode::serialize(&{
            let vec = bincode::serialize(&transaction).map_err(Error::Bincode)?;
            match node
                .blockchain
                .pending_transactions_push(transaction, node.args.time_delta)
            {
                Ok(()) => {
                    if let Err(e) = node.p2p.gossipsub_publish("transaction", vec) {
                        error!(?e);
                    }
                    "success".to_string()
                }
                Err(e) => {
                    error!(?e);
                    format!("{:?}", e)
                }
            }
        })
        .map_err(Error::Bincode)
    }
    pub fn stake(node: &mut Node, stake: Stake) -> Result<Vec<u8>, Error> {
        bincode::serialize(&{
            let vec = bincode::serialize(&stake).map_err(Error::Bincode)?;
            match node
                .blockchain
                .pending_stakes_push(stake, node.args.time_delta)
            {
                Ok(()) => {
                    if let Err(e) = node.p2p.gossipsub_publish("stake", vec) {
                        error!(?e);
                    }
                    "success".to_string()
                }
                Err(e) => {
                    error!(?e);
                    format!("{:?}", e)
                }
            }
        })
        .map_err(Error::Bincode)
    }
    pub fn address(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.key.as_ref().map(|x| x.address_bytes())).map_err(Error::Bincode)
    }
    pub fn ticks(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.ticks).map_err(Error::Bincode)
    }
    pub fn tree_size(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.tree.size()).map_err(Error::Bincode)
    }
    pub fn sync(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.sync).map_err(Error::Bincode)
    }
    pub fn random_queue(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.forks.unstable.stakers_n(8)).map_err(Error::Bincode)
    }
    pub fn unstable_hashes(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.forks.unstable.hashes.len()).map_err(Error::Bincode)
    }
    pub fn unstable_latest_hashes(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(
            &node
                .blockchain
                .forks
                .unstable
                .hashes
                .iter()
                .rev()
                .take(16)
                .collect::<Vec<_>>(),
        )
        .map_err(Error::Bincode)
    }
    pub fn unstable_stakers(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.forks.unstable.stakers.len()).map_err(Error::Bincode)
    }
    pub fn stable_hashes(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.forks.stable.hashes.len()).map_err(Error::Bincode)
    }
    pub fn stable_latest_hashes(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(
            &node
                .blockchain
                .forks
                .stable
                .hashes
                .iter()
                .rev()
                .take(16)
                .collect::<Vec<_>>(),
        )
        .map_err(Error::Bincode)
    }
    pub fn stable_stakers(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.blockchain.forks.stable.stakers.len()).map_err(Error::Bincode)
    }
}
