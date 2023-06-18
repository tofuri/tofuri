use crate::Args;
use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use tofuri_address::address;
use tofuri_api_core::Root;
use tofuri_api_core::Stake;
use tofuri_api_core::Transaction;
use tofuri_util::BLOCK_TIME;
use tracing::instrument;
#[instrument(skip_all)]
pub async fn root(State(args): State<Args>) -> impl IntoResponse {
    let cargo_pkg_name = tofuri_rpc::cargo_pkg_name(&args.rpc).await.unwrap();
    let cargo_pkg_version = tofuri_rpc::cargo_pkg_version(&args.rpc).await.unwrap();
    let cargo_pkg_repository = tofuri_rpc::cargo_pkg_repository(&args.rpc).await.unwrap();
    let git_hash = tofuri_rpc::git_hash(&args.rpc).await.unwrap();
    Json(Root {
        cargo_pkg_name,
        cargo_pkg_version,
        cargo_pkg_repository,
        git_hash,
    })
}
#[instrument(skip_all)]
pub async fn balance(State(args): State<Args>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let balance = parseint::to_string::<18>(
        tofuri_rpc::balance(&args.rpc, &address_bytes)
            .await
            .unwrap(),
    );
    Json(balance)
}
#[instrument(skip_all)]
pub async fn balance_pending_min(
    State(args): State<Args>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let balance_pending_min = parseint::to_string::<18>(
        tofuri_rpc::balance_pending_min(&args.rpc, &address_bytes)
            .await
            .unwrap(),
    );
    Json(balance_pending_min)
}
#[instrument(skip_all)]
pub async fn balance_pending_max(
    State(args): State<Args>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let balance_pending_max = parseint::to_string::<18>(
        tofuri_rpc::balance_pending_max(&args.rpc, &address_bytes)
            .await
            .unwrap(),
    );
    Json(balance_pending_max)
}
#[instrument(skip_all)]
pub async fn staked(State(args): State<Args>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let staked =
        parseint::to_string::<18>(tofuri_rpc::staked(&args.rpc, &address_bytes).await.unwrap());
    Json(staked)
}
#[instrument(skip_all)]
pub async fn staked_pending_min(
    State(args): State<Args>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let staked_pending_min = parseint::to_string::<18>(
        tofuri_rpc::staked_pending_min(&args.rpc, &address_bytes)
            .await
            .unwrap(),
    );
    Json(staked_pending_min)
}
#[instrument(skip_all)]
pub async fn staked_pending_max(
    State(args): State<Args>,
    address: Path<String>,
) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let staked_pending_max = parseint::to_string::<18>(
        tofuri_rpc::staked_pending_max(&args.rpc, &address_bytes)
            .await
            .unwrap(),
    );
    Json(staked_pending_max)
}
#[instrument(skip_all)]
pub async fn height(State(args): State<Args>) -> impl IntoResponse {
    let height = tofuri_rpc::height(&args.rpc).await.unwrap();
    Json(height)
}
#[instrument(skip_all)]
pub async fn height_by_hash(State(args): State<Args>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let height = tofuri_rpc::height_by_hash(&args.rpc, &hash).await.unwrap();
    Json(height)
}
#[instrument(skip_all)]
pub async fn block_latest(State(args): State<Args>) -> impl IntoResponse {
    let block_a = tofuri_rpc::block_latest(&args.rpc).await.unwrap();
    let block = tofuri_api_util::block(&block_a);
    Json(block)
}
#[instrument(skip_all)]
pub async fn hash_by_height(State(args): State<Args>, height: Path<String>) -> impl IntoResponse {
    let height: usize = height.parse().unwrap();
    let hash = hex::encode(
        tofuri_rpc::hash_by_height(&args.rpc, &height)
            .await
            .unwrap(),
    );
    Json(hash)
}
#[instrument(skip_all)]
pub async fn block_by_hash(State(args): State<Args>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let block_a = tofuri_rpc::block_by_hash(&args.rpc, &hash).await.unwrap();
    let block = tofuri_api_util::block(&block_a);
    Json(block)
}
#[instrument(skip_all)]
pub async fn transaction_by_hash(
    State(args): State<Args>,
    hash: Path<String>,
) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let transaction_a = tofuri_rpc::transaction_by_hash(&args.rpc, &hash)
        .await
        .unwrap();
    let transaction = tofuri_api_util::transaction(&transaction_a);
    Json(transaction)
}
#[instrument(skip_all)]
pub async fn stake_by_hash(State(args): State<Args>, hash: Path<String>) -> impl IntoResponse {
    let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let stake_a = tofuri_rpc::stake_by_hash(&args.rpc, &hash).await.unwrap();
    let stake = tofuri_api_util::stake(&stake_a);
    Json(stake)
}
#[instrument(skip_all)]
pub async fn peers(State(args): State<Args>) -> impl IntoResponse {
    let peers = tofuri_rpc::peers(&args.rpc).await.unwrap();
    Json(peers)
}
#[instrument(skip_all)]
pub async fn peer(State(args): State<Args>, Path(ip_addr): Path<String>) -> impl IntoResponse {
    tofuri_rpc::peer(&args.rpc, &ip_addr.parse().unwrap())
        .await
        .unwrap();
    Json(true)
}
#[instrument(skip_all)]
pub async fn transaction(
    State(args): State<Args>,
    Json(transaction): Json<Transaction>,
) -> impl IntoResponse {
    let transaction_b = tofuri_api_util::transaction_b(&transaction).unwrap();
    let status = tofuri_rpc::transaction(&args.rpc, &transaction_b)
        .await
        .unwrap();
    Json(status)
}
#[instrument(skip_all)]
pub async fn stake(State(args): State<Args>, Json(stake): Json<Stake>) -> impl IntoResponse {
    let stake_b = tofuri_api_util::stake_b(&stake).unwrap();
    let status = tofuri_rpc::stake(&args.rpc, &stake_b).await.unwrap();
    Json(status)
}
#[instrument(skip_all)]
pub async fn cargo_pkg_name(State(args): State<Args>) -> impl IntoResponse {
    let cargo_pkg_name = tofuri_rpc::cargo_pkg_name(&args.rpc).await.unwrap();
    Json(cargo_pkg_name)
}
#[instrument(skip_all)]
pub async fn cargo_pkg_version(State(args): State<Args>) -> impl IntoResponse {
    let cargo_pkg_version = tofuri_rpc::cargo_pkg_version(&args.rpc).await.unwrap();
    Json(cargo_pkg_version)
}
#[instrument(skip_all)]
pub async fn cargo_pkg_repository(State(args): State<Args>) -> impl IntoResponse {
    let cargo_pkg_repository = tofuri_rpc::cargo_pkg_repository(&args.rpc).await.unwrap();
    Json(cargo_pkg_repository)
}
#[instrument(skip_all)]
pub async fn git_hash(State(args): State<Args>) -> impl IntoResponse {
    let git_hash = tofuri_rpc::git_hash(&args.rpc).await.unwrap();
    Json(git_hash)
}
#[instrument(skip_all)]
pub async fn address(State(args): State<Args>) -> impl IntoResponse {
    let address = tofuri_rpc::address(&args.rpc).await.unwrap();
    let address = address.map(|x| address::encode(&x));
    Json(address)
}
#[instrument(skip_all)]
pub async fn ticks(State(args): State<Args>) -> impl IntoResponse {
    let ticks = tofuri_rpc::ticks(&args.rpc).await.unwrap();
    Json(ticks)
}
#[instrument(skip_all)]
pub async fn time(State(args): State<Args>) -> impl IntoResponse {
    let time = tofuri_rpc::time(&args.rpc).await.unwrap();
    Json(time)
}
#[instrument(skip_all)]
pub async fn tree_size(State(args): State<Args>) -> impl IntoResponse {
    let tree_size = tofuri_rpc::tree_size(&args.rpc).await.unwrap();
    Json(tree_size)
}
#[instrument(skip_all)]
pub async fn sync(State(args): State<Args>) -> impl IntoResponse {
    let sync = tofuri_rpc::sync(&args.rpc).await.unwrap();
    Json(sync)
}
#[instrument(skip_all)]
pub async fn random_queue(State(args): State<Args>) -> impl IntoResponse {
    let random_queue = tofuri_rpc::random_queue(&args.rpc).await.unwrap();
    let random_queue: Vec<String> = random_queue.iter().map(address::encode).collect();
    Json(random_queue)
}
#[instrument(skip_all)]
pub async fn unstable_hashes(State(args): State<Args>) -> impl IntoResponse {
    let unstable_hashes = tofuri_rpc::unstable_hashes(&args.rpc).await.unwrap();
    Json(unstable_hashes)
}
#[instrument(skip_all)]
pub async fn unstable_latest_hashes(State(args): State<Args>) -> impl IntoResponse {
    let unstable_latest_hashes = tofuri_rpc::unstable_latest_hashes(&args.rpc).await.unwrap();
    let unstable_latest_hashes: Vec<String> =
        unstable_latest_hashes.iter().map(hex::encode).collect();
    Json(unstable_latest_hashes)
}
#[instrument(skip_all)]
pub async fn unstable_stakers(State(args): State<Args>) -> impl IntoResponse {
    let unstable_stakers = tofuri_rpc::unstable_stakers(&args.rpc).await.unwrap();
    Json(unstable_stakers)
}
#[instrument(skip_all)]
pub async fn stable_hashes(State(args): State<Args>) -> impl IntoResponse {
    let stable_hashes = tofuri_rpc::stable_hashes(&args.rpc).await.unwrap();
    Json(stable_hashes)
}
#[instrument(skip_all)]
pub async fn stable_latest_hashes(State(args): State<Args>) -> impl IntoResponse {
    let stable_latest_hashes = tofuri_rpc::stable_latest_hashes(&args.rpc).await.unwrap();
    let stable_latest_hashes: Vec<String> = stable_latest_hashes.iter().map(hex::encode).collect();
    Json(stable_latest_hashes)
}
#[instrument(skip_all)]
pub async fn stable_stakers(State(args): State<Args>) -> impl IntoResponse {
    let stable_stakers = tofuri_rpc::stable_stakers(&args.rpc).await.unwrap();
    Json(stable_stakers)
}
#[instrument(skip_all)]
pub async fn sync_remaining(State(args): State<Args>) -> impl IntoResponse {
    let sync = tofuri_rpc::sync(&args.rpc).await.unwrap();
    if sync.completed {
        return Json(0.0);
    }
    if !sync.downloading() {
        return Json(-1.0);
    }
    let block_a = tofuri_rpc::block_latest(&args.rpc).await.unwrap();
    let mut diff = tofuri_util::timestamp().saturating_sub(block_a.timestamp) as f32;
    diff /= BLOCK_TIME as f32;
    diff /= sync.bps;
    Json(diff)
}
