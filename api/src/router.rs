use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use multiaddr::Multiaddr;
use pea_address::address;
use pea_core::*;
use serde_json::Value;
pub async fn root() -> &'static str {
    "Hello, World!"
}
pub async fn balance(address: Path<String>) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let height = pea_api_client::balance("localhost:9332", &address_bytes).await.unwrap();
    (StatusCode::OK, Json(pea_int::to_string(height)))
}
pub async fn staked(address: Path<String>) -> impl IntoResponse {
    let address_bytes = address::decode(&address).unwrap();
    let staked = pea_api_client::staked("localhost:9332", &address_bytes).await.unwrap();
    (StatusCode::OK, Json(pea_int::to_string(staked)))
}
pub async fn height() -> impl IntoResponse {
    let height = pea_api_client::height("localhost:9332").await.unwrap();
    (StatusCode::OK, Json(height))
}
pub async fn height_by_hash(hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    println!("{:?}", hash);
    let height = pea_api_client::height_by_hash("localhost:9332", &hash).await.unwrap();
    (StatusCode::OK, Json(height))
}
pub async fn block_latest() -> impl IntoResponse {
    let block_a = pea_api_client::block_latest("localhost:9332").await;
    println!("{:?}", block_a);
    let block_a = block_a.unwrap();
    (
        StatusCode::OK,
        Json(pea_api_core::external::Block {
            hash: hex::encode(block_a.hash),
            previous_hash: hex::encode(block_a.previous_hash),
            timestamp: block_a.timestamp,
            beta: hex::encode(block_a.beta),
            pi: hex::encode(block_a.pi),
            forger_address: address::encode(&block_a.input_address()),
            signature: hex::encode(block_a.signature),
            transactions: block_a.transactions.iter().map(|x| hex::encode(x.hash)).collect(),
            stakes: block_a.stakes.iter().map(|x| hex::encode(x.hash)).collect(),
        }),
    )
}
pub async fn hash_by_height(height: Path<String>) -> impl IntoResponse {
    let height: usize = height.parse().unwrap();
    let hash = pea_api_client::hash_by_height("localhost:9332", &height).await.unwrap();
    (StatusCode::OK, Json(hex::encode(hash)))
}
pub async fn block_by_hash(hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let block_a = pea_api_client::block_by_hash("localhost:9332", &hash).await.unwrap();
    (StatusCode::OK, Json(block_a))
}
pub async fn transaction_by_hash(hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let transaction_a = pea_api_client::transaction_by_hash("localhost:9332", &hash).await.unwrap();
    (StatusCode::OK, Json(transaction_a))
}
pub async fn stake_by_hash(hash: Path<String>) -> impl IntoResponse {
    let hash: Hash = hex::decode(hash.clone()).unwrap().try_into().unwrap();
    let stake_a = pea_api_client::stake_by_hash("localhost:9332", &hash).await.unwrap();
    (StatusCode::OK, Json(stake_a))
}
pub async fn peers() -> impl IntoResponse {
    let peers = pea_api_client::peers("localhost:9332").await.unwrap();
    (StatusCode::OK, Json(peers))
}
pub async fn peer(a: Path<String>, b: Path<String>, c: Path<String>, d: Path<String>) -> impl IntoResponse {
    let multiaddr: Multiaddr = format!("/{}/{}/{}/{}", a.as_str(), b.as_str(), c.as_str(), d.as_str()).parse().unwrap();
    pea_api_client::peer("localhost:9332", &multiaddr).await.unwrap();
    (StatusCode::OK, Json(()))
}
pub async fn transaction(Json(payload): Json<Value>) -> impl IntoResponse {
    (StatusCode::OK, Json(()))
    // let status = pea_api_client::transaction("localhost:9332").await.unwrap();
    // (StatusCode::OK, Json(status))
}
pub async fn stake() -> impl IntoResponse {
    (StatusCode::OK, Json(()))
    // let status = pea_api_client::stake("localhost:9332").await.unwrap();
    // (StatusCode::OK, Json(status))
}
