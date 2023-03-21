use colored::*;
use lazy_static::lazy_static;
use sha2::Digest;
use sha2::Sha256;
use std::time::Duration;
use tofuri_block::BlockB;
use tofuri_core::*;
use tofuri_stake::StakeB;
use tofuri_transaction::TransactionB;
use tokio::time::Instant;
use tokio::time::Interval;
use uint::construct_uint;
pub const GIT_HASH: &str = env!("GIT_HASH");
lazy_static! {
    pub static ref EMPTY_BLOCK_SIZE: usize = bincode::serialize(&BlockB::default()).unwrap().len();
    pub static ref TRANSACTION_SIZE: usize = bincode::serialize(&TransactionB::default()).unwrap().len();
    pub static ref STAKE_SIZE: usize = bincode::serialize(&StakeB::default()).unwrap().len();
}
construct_uint! {
    pub struct U256(4);
}
pub fn u256(hash: &Hash) -> U256 {
    U256::from_big_endian(hash)
}
pub fn u256_modulo(hash: &Hash, modulo: u128) -> u128 {
    (u256(hash) % modulo).as_u128()
}
pub fn hash_beta_n(beta: &Beta, n: u128) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(beta);
    hasher.update(n.to_be_bytes());
    hasher.finalize().into()
}
pub fn random(beta: &Beta, n: u128, modulo: u128) -> u128 {
    u256_modulo(&hash_beta_n(beta, n), modulo)
}
pub fn penalty(index: usize) -> u128 {
    if index == 0 {
        return 0;
    }
    COIN * 2u128.pow(index as u32 - 1)
}
pub fn timestamp() -> u32 {
    chrono::offset::Utc::now().timestamp() as u32
}
pub fn duration_to_string(seconds: u32, now: &str) -> String {
    if seconds == 0 {
        return now.to_string();
    }
    let mut string = "".to_string();
    let mut i = 0;
    for (str, num) in [
        ("week", seconds / 604800),
        ("day", seconds / 86400 % 7),
        ("hour", seconds / 3600 % 24),
        ("minute", seconds / 60 % 60),
        ("second", seconds % 60),
    ] {
        if num == 0 {
            continue;
        }
        if i == 1 {
            string.push_str(" and ");
        }
        string.push_str(&format!("{} {}{}", num, str, if num == 1 { "" } else { "s" }));
        if i == 1 {
            break;
        }
        i += 1;
    }
    string
}
pub fn elapsed(timestamp: u32, latest_block_timestamp: u32) -> bool {
    ELAPSED + timestamp < latest_block_timestamp
}
pub fn duration_until_next_tick(duration: Duration) -> Duration {
    let nanos = duration.as_nanos() as u64;
    Duration::from_nanos(nanos - chrono::offset::Utc::now().timestamp_nanos() as u64 % nanos)
}
pub fn interval_at(duration: Duration) -> Interval {
    let start = Instant::now() + duration_until_next_tick(duration);
    tokio::time::interval_at(start, duration)
}
pub fn build(cargo_pkg_name: &str, cargo_pkg_version: &str, cargo_pkg_repository: &str) -> String {
    format!(
        "\
{} = {{ version = \"{}\" }}
{}/tree/{}",
        cargo_pkg_name.yellow(),
        cargo_pkg_version.magenta(),
        cargo_pkg_repository.yellow(),
        GIT_HASH.magenta()
    )
}
