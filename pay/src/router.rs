use crate::Pay;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use decimal::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
pub async fn root() -> &'static str {
    ""
}
pub async fn charges(State(pay): State<Arc<Mutex<Pay>>>) -> impl IntoResponse {
    let pay = pay.lock().await;
    let charges = pay.get_charges();
    (StatusCode::OK, Json(charges))
}
pub async fn charge(State(pay): State<Arc<Mutex<Pay>>>, hash: Path<String>) -> impl IntoResponse {
    let hash = hex::decode(hash.as_str()).unwrap();
    let pay = pay.lock().await;
    let payment = pay.get_charge(&hash);
    (StatusCode::OK, Json(payment))
}
pub async fn charge_new(
    State(pay): State<Arc<Mutex<Pay>>>,
    amount: Path<String>,
) -> impl IntoResponse {
    let amount = u128::from_str::<18>(&amount).unwrap();
    let mut pay = pay.lock().await;
    let payment = pay.charge(amount).unwrap();
    (StatusCode::OK, Json(payment))
}
