use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use pea_address::address;
use pea_api_client::request;
use pea_api_core::internal::Data;
use pea_block::BlockA;
pub async fn root() -> &'static str {
    "Hello, World!"
}
pub async fn height() -> impl IntoResponse {
    let height: usize = bincode::deserialize(&request("localhost:9332", Data::Height, None).await.unwrap()).unwrap();
    (StatusCode::OK, Json(height))
}
pub async fn block_latest() -> impl IntoResponse {
    let block_a: BlockA = bincode::deserialize(&request("localhost:9332", Data::BlockLatest, None).await.unwrap()).unwrap();
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
