use super::Call;
use super::Client;
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
use stake::Stake;
use std::convert::TryInto;
use std::net::IpAddr;
use sync::Sync;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use transaction::Transaction;
pub async fn serve(client: Client, api: String) {
    let addr = api.parse().unwrap();
    info!(?addr, "api server listening on");
    let make_service = router(client).into_make_service();
    Server::bind(&addr).serve(make_service).await.unwrap();
}
fn router(client: Client) -> Router {
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
        .with_state(client)
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
async fn balance(State(client): State<Client>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(client.call::<u128>(Call::Balance(address_bytes)).await)
}
async fn balance_pending_min(
    State(client): State<Client>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(
        client
            .call::<u128>(Call::BalancePendingMin(address_bytes))
            .await,
    )
}
async fn balance_pending_max(
    State(client): State<Client>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(
        client
            .call::<u128>(Call::BalancePendingMax(address_bytes))
            .await,
    )
}
async fn staked(State(client): State<Client>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(client.call::<u128>(Call::Staked(address_bytes)).await)
}
async fn staked_pending_min(
    State(client): State<Client>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(
        client
            .call::<u128>(Call::StakedPendingMin(address_bytes))
            .await,
    )
}
async fn staked_pending_max(
    State(client): State<Client>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = public::decode(&address).unwrap();
    Json(
        client
            .call::<u128>(Call::StakedPendingMax(address_bytes))
            .await,
    )
}
async fn height(State(client): State<Client>) -> impl IntoResponse {
    Json(client.call::<usize>(Call::Height).await)
}
async fn height_by_hash(State(client): State<Client>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    Json(client.call::<usize>(Call::HeightByHash(hash)).await)
}
async fn block_latest(State(client): State<Client>) -> impl IntoResponse {
    let block = client.call::<Block>(Call::BlockLatest).await;
    let block_hex: BlockHex = block.try_into().unwrap();
    Json(block_hex)
}
async fn hash_by_height(State(client): State<Client>, height: Path<String>) -> impl IntoResponse {
    let height: usize = height.parse().unwrap();
    let hash = client.call::<[u8; 32]>(Call::HashByHeight(height)).await;
    let hash_hex = hex::encode(hash);
    Json(hash_hex)
}
async fn block_by_hash(State(client): State<Client>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let block = client.call::<Block>(Call::BlockByHash(hash)).await;
    let block_hex: BlockHex = block.try_into().unwrap();
    Json(block_hex)
}
async fn transaction_by_hash(
    State(client): State<Client>,
    hash: Path<String>,
) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let transaction = client
        .call::<Transaction>(Call::TransactionByHash(hash))
        .await;
    let transaction_hex: TransactionHex = transaction.try_into().unwrap();
    Json(transaction_hex)
}
async fn stake_by_hash(State(client): State<Client>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let stake = client.call::<Stake>(Call::StakeByHash(hash)).await;
    let stake_hex: StakeHex = stake.try_into().unwrap();
    Json(stake_hex)
}
async fn peers(State(client): State<Client>) -> impl IntoResponse {
    Json(client.call::<Vec<IpAddr>>(Call::Peers).await)
}
async fn peer(State(client): State<Client>, Path(ip_addr): Path<String>) -> impl IntoResponse {
    let ip_addr = ip_addr.parse().unwrap();
    Json(client.call::<bool>(Call::Peer(ip_addr)).await)
}
async fn transaction(
    State(client): State<Client>,
    Json(transaction): Json<TransactionHex>,
) -> impl IntoResponse {
    let transaction: Transaction = transaction.try_into().unwrap();
    Json(client.call::<bool>(Call::Transaction(transaction)).await)
}
async fn stake(State(client): State<Client>, Json(stake): Json<StakeHex>) -> impl IntoResponse {
    let stake: Stake = stake.try_into().unwrap();
    Json(client.call::<bool>(Call::Stake(stake)).await)
}
async fn address(State(client): State<Client>) -> impl IntoResponse {
    Json(public::encode(
        &client.call::<[u8; 20]>(Call::Address).await,
    ))
}
async fn ticks(State(client): State<Client>) -> impl IntoResponse {
    Json(client.call::<usize>(Call::Ticks).await)
}
async fn time() -> impl IntoResponse {
    Json(chrono::offset::Utc::now().timestamp_millis())
}
async fn tree_size(State(client): State<Client>) -> impl IntoResponse {
    Json(client.call::<usize>(Call::TreeSize).await)
}
async fn sync(State(client): State<Client>) -> impl IntoResponse {
    let sync = client.call::<Sync>(Call::Sync).await;
    Json(sync)
}
async fn random_queue(State(client): State<Client>) -> impl IntoResponse {
    Json(
        client
            .call::<Vec<[u8; 20]>>(Call::RandomQueue)
            .await
            .iter()
            .map(public::encode)
            .collect::<Vec<_>>(),
    )
}
async fn unstable_hashes(State(client): State<Client>) -> impl IntoResponse {
    Json(client.call::<usize>(Call::UnstableHashes).await)
}
async fn unstable_latest_hashes(State(client): State<Client>) -> impl IntoResponse {
    Json(
        client
            .call::<Vec<[u8; 32]>>(Call::UnstableLatestHashes)
            .await
            .iter()
            .map(hex::encode)
            .collect::<Vec<_>>(),
    )
}
async fn unstable_stakers(State(client): State<Client>) -> impl IntoResponse {
    Json(client.call::<usize>(Call::UnstableStakers).await)
}
async fn stable_hashes(State(client): State<Client>) -> impl IntoResponse {
    Json(client.call::<usize>(Call::StableHashes).await)
}
async fn stable_latest_hashes(State(client): State<Client>) -> impl IntoResponse {
    Json(
        client
            .call::<Vec<[u8; 32]>>(Call::StableLatestHashes)
            .await
            .iter()
            .map(hex::encode)
            .collect::<Vec<_>>(),
    )
}
async fn stable_stakers(State(client): State<Client>) -> impl IntoResponse {
    Json(client.call::<usize>(Call::StableStakers).await)
}
async fn sync_remaining(State(client): State<Client>) -> impl IntoResponse {
    let sync = client.call::<Sync>(Call::Sync).await;
    if sync.completed {
        return Json(0.0);
    }
    if !sync.downloading() {
        return Json(-1.0);
    }
    let block = client.call::<Block>(Call::BlockLatest).await;
    let mut diff = (Utc::now().timestamp() as u32).saturating_sub(block.timestamp) as f32;
    diff /= BLOCK_TIME as f32;
    diff /= sync.bps;
    Json(diff)
}
