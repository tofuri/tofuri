use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use multiaddr::Multiaddr;
use pea_address::address;
use pea_api_core::Stake;
use pea_api_core::Transaction;
use pea_core::*;
use serde_json::Value;
pub const API: &str = "localhost:9332";
pub async fn root() -> &'static str {
    "Hello, World!"
}
pub async fn balance(address: Path<String>) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let height = pea_api_internal::balance(API, &address_bytes).await.unwrap();
    (StatusCode::OK, Json(pea_int::to_string(height)))
}
pub async fn staked(address: Path<String>) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let staked = pea_api_internal::staked(API, &address_bytes).await.unwrap();
    (StatusCode::OK, Json(pea_int::to_string(staked)))
}
pub async fn height() -> impl IntoResponse {
    let height = pea_api_internal::height(API).await.unwrap();
    (StatusCode::OK, Json(height))
}
pub async fn height_by_hash(hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let height = pea_api_internal::height_by_hash(API, &hash).await.unwrap();
    (StatusCode::OK, Json(height))
}
pub async fn block_latest() -> impl IntoResponse {
    let block_a = pea_api_internal::block_latest(API).await.unwrap();
    (StatusCode::OK, Json(pea_api_util::block_json(&block_a)))
}
pub async fn hash_by_height(height: Path<String>) -> impl IntoResponse {
    let height: usize = height.parse().unwrap();
    let hash = pea_api_internal::hash_by_height(API, &height).await.unwrap();
    (StatusCode::OK, Json(hex::encode(hash)))
}
pub async fn block_by_hash(hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let block_a = pea_api_internal::block_by_hash(API, &hash).await.unwrap();
    (StatusCode::OK, Json(pea_api_util::block_json(&block_a)))
}
pub async fn transaction_by_hash(hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let transaction_a = pea_api_internal::transaction_by_hash(API, &hash).await.unwrap();
    (StatusCode::OK, Json(pea_api_util::transaction_json(&transaction_a)))
}
pub async fn stake_by_hash(hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let stake_a = pea_api_internal::stake_by_hash(API, &hash).await.unwrap();
    (StatusCode::OK, Json(pea_api_util::stake_json(&stake_a)))
}
pub async fn peers() -> impl IntoResponse {
    let peers = pea_api_internal::peers(API).await.unwrap();
    (StatusCode::OK, Json(peers))
}
pub async fn peer(a: Path<String>, b: Path<String>, c: Path<String>, d: Path<String>) -> impl IntoResponse {
    let multiaddr: Multiaddr = format!("/{}/{}/{}/{}", a.as_str(), b.as_str(), c.as_str(), d.as_str()).parse().unwrap();
    pea_api_internal::peer(API, &multiaddr).await.unwrap();
    (StatusCode::OK, Json(()))
}
pub async fn transaction(Json(transaction): Json<Transaction>) -> impl IntoResponse {
    let transaction_b = pea_api_util::transaction_b(&transaction).unwrap();
    let status = pea_api_internal::transaction(API, &transaction_b).await.unwrap();
    (StatusCode::OK, Json(status))
}
pub async fn stake(Json(stake): Json<Stake>) -> impl IntoResponse {
    let stake_b = pea_api_util::stake_b(&stake).unwrap();
    let status = pea_api_internal::stake(API, &stake_b).await.unwrap();
    (StatusCode::OK, Json(status))
}
