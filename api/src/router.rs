use crate::Args;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use multiaddr::Multiaddr;
use pea_address::address;
use pea_api_core::Stake;
use pea_api_core::Transaction;
use pea_core::*;
pub async fn root(State(args): State<Args>) -> impl IntoResponse {
    let cargo_pkg_name = pea_api_internal::cargo_pkg_name(&args.api_internal).await.unwrap();
    let cargo_pkg_version = pea_api_internal::cargo_pkg_version(&args.api_internal).await.unwrap();
    let cargo_pkg_repository = pea_api_internal::cargo_pkg_repository(&args.api_internal).await.unwrap();
    let git_hash = pea_api_internal::git_hash(&args.api_internal).await.unwrap();
    (
        StatusCode::OK,
        format!(
            "\
{} = {{ version = \"{}\" }}
{}/tree/{}",
            cargo_pkg_name, cargo_pkg_version, cargo_pkg_repository, git_hash,
        ),
    )
}
pub async fn balance(State(args): State<Args>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let balance = pea_int::to_string(pea_api_internal::balance(&args.api_internal, &address_bytes).await.unwrap());
    (StatusCode::OK, Json(balance))
}
pub async fn staked(State(args): State<Args>, address: Path<String>) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let staked = pea_int::to_string(pea_api_internal::staked(&args.api_internal, &address_bytes).await.unwrap());
    (StatusCode::OK, Json(staked))
}
pub async fn height(State(args): State<Args>) -> impl IntoResponse {
    let height = pea_api_internal::height(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(height))
}
pub async fn height_by_hash(State(args): State<Args>, hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let height = pea_api_internal::height_by_hash(&args.api_internal, &hash).await.unwrap();
    (StatusCode::OK, Json(height))
}
pub async fn block_latest(State(args): State<Args>) -> impl IntoResponse {
    let block_a = pea_api_internal::block_latest(&args.api_internal).await.unwrap();
    let block = pea_api_util::block(&block_a);
    (StatusCode::OK, Json(block))
}
pub async fn hash_by_height(State(args): State<Args>, height: Path<String>) -> impl IntoResponse {
    let height: usize = height.parse().unwrap();
    let hash = hex::encode(pea_api_internal::hash_by_height(&args.api_internal, &height).await.unwrap());
    (StatusCode::OK, Json(hash))
}
pub async fn block_by_hash(State(args): State<Args>, hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let block_a = pea_api_internal::block_by_hash(&args.api_internal, &hash).await.unwrap();
    let block = pea_api_util::block(&block_a);
    (StatusCode::OK, Json(block))
}
pub async fn transaction_by_hash(State(args): State<Args>, hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let transaction_a = pea_api_internal::transaction_by_hash(&args.api_internal, &hash).await.unwrap();
    let transaction = pea_api_util::transaction(&transaction_a);
    (StatusCode::OK, Json(transaction))
}
pub async fn stake_by_hash(State(args): State<Args>, hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let stake_a = pea_api_internal::stake_by_hash(&args.api_internal, &hash).await.unwrap();
    let stake = pea_api_util::stake(&stake_a);
    (StatusCode::OK, Json(stake))
}
pub async fn peers(State(args): State<Args>) -> impl IntoResponse {
    let peers = pea_api_internal::peers(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(peers))
}
pub async fn peer(State(args): State<Args>, a: Path<String>, b: Path<String>, c: Path<String>, d: Path<String>) -> impl IntoResponse {
    let multiaddr: Multiaddr = format!("/{}/{}/{}/{}", a.as_str(), b.as_str(), c.as_str(), d.as_str()).parse().unwrap();
    pea_api_internal::peer(&args.api_internal, &multiaddr).await.unwrap();
    (StatusCode::OK, Json(()))
}
pub async fn transaction(State(args): State<Args>, Json(transaction): Json<Transaction>) -> impl IntoResponse {
    let transaction_b = pea_api_util::transaction_b(&transaction).unwrap();
    let status = pea_api_internal::transaction(&args.api_internal, &transaction_b).await.unwrap();
    (StatusCode::OK, Json(status))
}
pub async fn stake(State(args): State<Args>, Json(stake): Json<Stake>) -> impl IntoResponse {
    let stake_b = pea_api_util::stake_b(&stake).unwrap();
    let status = pea_api_internal::stake(&args.api_internal, &stake_b).await.unwrap();
    (StatusCode::OK, Json(status))
}
pub async fn cargo_pkg_name(State(args): State<Args>) -> impl IntoResponse {
    let cargo_pkg_name = pea_api_internal::cargo_pkg_name(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(cargo_pkg_name))
}
pub async fn cargo_pkg_version(State(args): State<Args>) -> impl IntoResponse {
    let cargo_pkg_version = pea_api_internal::cargo_pkg_version(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(cargo_pkg_version))
}
pub async fn cargo_pkg_repository(State(args): State<Args>) -> impl IntoResponse {
    let cargo_pkg_repository = pea_api_internal::cargo_pkg_repository(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(cargo_pkg_repository))
}
pub async fn git_hash(State(args): State<Args>) -> impl IntoResponse {
    let git_hash = pea_api_internal::git_hash(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(git_hash))
}
pub async fn address(State(args): State<Args>) -> impl IntoResponse {
    let address = pea_api_internal::address(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(address))
}
pub async fn ticks(State(args): State<Args>) -> impl IntoResponse {
    let ticks = pea_api_internal::ticks(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(ticks))
}
pub async fn tps(State(args): State<Args>) -> impl IntoResponse {
    let tps = pea_api_internal::tps(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(tps))
}
pub async fn lag(State(args): State<Args>) -> impl IntoResponse {
    let lag = pea_api_internal::lag(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(lag))
}
pub async fn time(State(args): State<Args>) -> impl IntoResponse {
    let time = pea_api_internal::time(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(time))
}
pub async fn tree_size(State(args): State<Args>) -> impl IntoResponse {
    let tree_size = pea_api_internal::tree_size(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(tree_size))
}
pub async fn sync(State(args): State<Args>) -> impl IntoResponse {
    let sync = pea_api_internal::sync(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(sync))
}
pub async fn random_queue(State(args): State<Args>) -> impl IntoResponse {
    let random_queue = pea_api_internal::random_queue(&args.api_internal).await.unwrap();
    let random_queue: Vec<String> = random_queue.iter().map(address::encode).collect();
    (StatusCode::OK, Json(random_queue))
}
pub async fn dynamic_hashes(State(args): State<Args>) -> impl IntoResponse {
    let dynamic_hashes = pea_api_internal::dynamic_hashes(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(dynamic_hashes))
}
pub async fn dynamic_latest_hashes(State(args): State<Args>) -> impl IntoResponse {
    let dynamic_latest_hashes = pea_api_internal::dynamic_latest_hashes(&args.api_internal).await.unwrap();
    let dynamic_latest_hashes: Vec<String> = dynamic_latest_hashes.iter().map(hex::encode).collect();
    (StatusCode::OK, Json(dynamic_latest_hashes))
}
pub async fn dynamic_stakers(State(args): State<Args>) -> impl IntoResponse {
    let dynamic_stakers = pea_api_internal::dynamic_stakers(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(dynamic_stakers))
}
pub async fn trusted_hashes(State(args): State<Args>) -> impl IntoResponse {
    let trusted_hashes = pea_api_internal::trusted_hashes(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(trusted_hashes))
}
pub async fn trusted_latest_hashes(State(args): State<Args>) -> impl IntoResponse {
    let trusted_latest_hashes = pea_api_internal::trusted_latest_hashes(&args.api_internal).await.unwrap();
    let trusted_latest_hashes: Vec<String> = trusted_latest_hashes.iter().map(hex::encode).collect();
    (StatusCode::OK, Json(trusted_latest_hashes))
}
pub async fn trusted_stakers(State(args): State<Args>) -> impl IntoResponse {
    let trusted_stakers = pea_api_internal::trusted_stakers(&args.api_internal).await.unwrap();
    (StatusCode::OK, Json(trusted_stakers))
}
