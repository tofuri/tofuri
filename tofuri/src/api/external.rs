use super::internal::Internal;
use super::Get;
use crate::CARGO_PKG_NAME;
use crate::CARGO_PKG_REPOSITORY;
use crate::CARGO_PKG_VERSION;
use crate::GIT_HASH;
use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use axum::Server;
use chrono::offset::Utc;
use hex;
use std::convert::TryInto;
use std::net::IpAddr;
use tofuri_address::public;
use tofuri_api::BlockHex;
use tofuri_api::Root;
use tofuri_api::StakeHex;
use tofuri_api::TransactionHex;
use tofuri_block::Block;
use tofuri_blockchain::fork::BLOCK_TIME;
use tofuri_blockchain::sync::Sync;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
pub async fn serve(internal: Internal, api: String) {
    let addr = api.parse().unwrap();
    info!(?addr, "API listening");
    let make_service = router(internal).into_make_service();
    Server::bind(&addr).serve(make_service).await.unwrap();
}
fn router(internal: Internal) -> Router {
    let cors = CorsLayer::permissive();
    let trace = TraceLayer::new_for_http();
    Router::new()
        .route("/", get(root))
        .route("/balance/:address", get(balance))
        .route("/balance_pending_min/:address", get(balance_pending_min))
        .route("/balance_pending_max/:address", get(balance_pending_max))
        .route("/staked/:address", get(staked))
        .route("/staked_pending_min/:address", get(staked_pending_min))
        .route("/staked_pending_max/:address", get(staked_pending_max))
        .route("/height", get(height))
        .route("/height/:hash", get(height_by_hash))
        .route("/block", get(block_latest))
        .route("/hash/:height", get(hash_by_height))
        .route("/block/:hash", get(block_by_hash))
        .route("/transaction/:hash", get(transaction_by_hash))
        .route("/stake/:hash", get(stake_by_hash))
        .route("/peers", get(peers))
        .route("/peer/:ip_addr", get(peer))
        .route("/transaction", post(transaction))
        .route("/stake", post(stake))
        .route("/cargo_pkg_name", get(cargo_pkg_name))
        .route("/cargo_pkg_version", get(cargo_pkg_version))
        .route("/cargo_pkg_repository", get(cargo_pkg_repository))
        .route("/git_hash", get(git_hash))
        .route("/address", get(address))
        .route("/ticks", get(ticks))
        .route("/time", get(time))
        .route("/tree_size", get(tree_size))
        .route("/sync", get(sync))
        .route("/random_queue", get(random_queue))
        .route("/unstable_hashes", get(unstable_hashes))
        .route("/unstable_latest_hashes", get(unstable_latest_hashes))
        .route("/unstable_stakers", get(unstable_stakers))
        .route("/stable_hashes", get(stable_hashes))
        .route("/stable_latest_hashes", get(stable_latest_hashes))
        .route("/stable_stakers", get(stable_stakers))
        .route("/sync_remaining", get(sync_remaining))
        .layer(trace)
        .layer(cors)
        .with_state(internal)
}
async fn root() -> impl IntoResponse {
    Json(Root {
        cargo_pkg_name: CARGO_PKG_NAME.to_string(),
        cargo_pkg_version: CARGO_PKG_VERSION.to_string(),
        cargo_pkg_repository: CARGO_PKG_REPOSITORY.to_string(),
        git_hash: GIT_HASH.to_string(),
    })
}
async fn cargo_pkg_name() -> impl IntoResponse {
    Json(CARGO_PKG_NAME)
}
async fn cargo_pkg_version() -> impl IntoResponse {
    Json(CARGO_PKG_VERSION)
}
async fn cargo_pkg_repository() -> impl IntoResponse {
    Json(CARGO_PKG_REPOSITORY)
}
async fn git_hash() -> impl IntoResponse {
    Json(GIT_HASH)
}
async fn balance(State(s): State<Internal>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(s.get::<u128>(Get::Balance(address_bytes)).await)
}
async fn balance_pending_min(
    State(s): State<Internal>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(s.get::<u128>(Get::BalancePendingMin(address_bytes)).await)
}
async fn balance_pending_max(
    State(s): State<Internal>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(s.get::<u128>(Get::BalancePendingMax(address_bytes)).await)
}
async fn staked(State(s): State<Internal>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(s.get::<u128>(Get::Staked(address_bytes)).await)
}
async fn staked_pending_min(State(s): State<Internal>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(s.get::<u128>(Get::StakedPendingMin(address_bytes)).await)
}
async fn staked_pending_max(State(s): State<Internal>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(s.get::<u128>(Get::StakedPendingMax(address_bytes)).await)
}
async fn height(State(s): State<Internal>) -> impl IntoResponse {
    Json(s.get::<usize>(Get::Height).await)
}
async fn height_by_hash(State(s): State<Internal>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    Json(s.get::<usize>(Get::HeightByHash(hash)).await)
}
async fn block_latest(State(s): State<Internal>) -> impl IntoResponse {
    let block = s.get::<Block>(Get::BlockLatest).await;
    let block_hex: BlockHex = block.try_into().unwrap();
    Json(block_hex)
}
async fn hash_by_height(State(s): State<Internal>, height: Path<String>) -> impl IntoResponse {
    let height: usize = height.parse().unwrap();
    let hash = s.get::<[u8; 32]>(Get::HashByHeight(height)).await;
    let hash_hex = hex::encode(hash);
    Json(hash_hex)
}
async fn block_by_hash(State(s): State<Internal>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let block = s.get::<Block>(Get::BlockByHash(hash)).await;
    let block_hex: BlockHex = block.try_into().unwrap();
    Json(block_hex)
}
async fn transaction_by_hash(State(s): State<Internal>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let transaction = s.get::<Transaction>(Get::TransactionByHash(hash)).await;
    let transaction_hex: TransactionHex = transaction.try_into().unwrap();
    Json(transaction_hex)
}
async fn stake_by_hash(State(s): State<Internal>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let stake = s.get::<Stake>(Get::StakeByHash(hash)).await;
    let stake_hex: StakeHex = stake.try_into().unwrap();
    Json(stake_hex)
}
async fn peers(State(s): State<Internal>) -> impl IntoResponse {
    Json(s.get::<Vec<IpAddr>>(Get::Peers).await)
}
async fn peer(State(s): State<Internal>, Path(ip_addr): Path<String>) -> impl IntoResponse {
    let ip_addr = ip_addr.parse().unwrap();
    Json(s.get::<bool>(Get::Peer(ip_addr)).await)
}
async fn transaction(
    State(s): State<Internal>,
    Json(transaction): Json<TransactionHex>,
) -> impl IntoResponse {
    let transaction: Transaction = transaction.try_into().unwrap();
    Json(s.get::<bool>(Get::Transaction(transaction)).await)
}
async fn stake(State(s): State<Internal>, Json(stake): Json<StakeHex>) -> impl IntoResponse {
    let stake: Stake = stake.try_into().unwrap();
    Json(s.get::<bool>(Get::Stake(stake)).await)
}
async fn address(State(s): State<Internal>) -> impl IntoResponse {
    Json(public::encode(&s.get::<[u8; 20]>(Get::Address).await))
}
async fn ticks(State(s): State<Internal>) -> impl IntoResponse {
    Json(s.get::<usize>(Get::Ticks).await)
}
async fn time() -> impl IntoResponse {
    Json(chrono::offset::Utc::now().timestamp_millis())
}
async fn tree_size(State(s): State<Internal>) -> impl IntoResponse {
    Json(s.get::<usize>(Get::TreeSize).await)
}
async fn sync(State(s): State<Internal>) -> impl IntoResponse {
    let sync = s.get::<Sync>(Get::Sync).await;
    Json(sync)
}
async fn random_queue(State(s): State<Internal>) -> impl IntoResponse {
    Json(
        s.get::<Vec<[u8; 20]>>(Get::RandomQueue)
            .await
            .iter()
            .map(public::encode)
            .collect::<Vec<_>>(),
    )
}
async fn unstable_hashes(State(s): State<Internal>) -> impl IntoResponse {
    Json(s.get::<usize>(Get::UnstableHashes).await)
}
async fn unstable_latest_hashes(State(s): State<Internal>) -> impl IntoResponse {
    Json(
        s.get::<Vec<[u8; 32]>>(Get::UnstableLatestHashes)
            .await
            .iter()
            .map(hex::encode)
            .collect::<Vec<_>>(),
    )
}
async fn unstable_stakers(State(s): State<Internal>) -> impl IntoResponse {
    Json(s.get::<usize>(Get::UnstableStakers).await)
}
async fn stable_hashes(State(s): State<Internal>) -> impl IntoResponse {
    Json(s.get::<usize>(Get::StableHashes).await)
}
async fn stable_latest_hashes(State(s): State<Internal>) -> impl IntoResponse {
    Json(
        s.get::<Vec<[u8; 32]>>(Get::StableLatestHashes)
            .await
            .iter()
            .map(hex::encode)
            .collect::<Vec<_>>(),
    )
}
async fn stable_stakers(State(s): State<Internal>) -> impl IntoResponse {
    Json(s.get::<usize>(Get::StableStakers).await)
}
async fn sync_remaining(State(s): State<Internal>) -> impl IntoResponse {
    let sync = s.get::<Sync>(Get::Sync).await;
    if sync.completed {
        return Json(0.0);
    }
    if !sync.downloading() {
        return Json(-1.0);
    }
    let block = s.get::<Block>(Get::BlockLatest).await;
    let mut diff = (Utc::now().timestamp() as u32).saturating_sub(block.timestamp) as f32;
    diff /= BLOCK_TIME as f32;
    diff /= sync.bps;
    Json(diff)
}
