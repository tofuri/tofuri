use colored::*;
use lazy_static::lazy_static;
use multiaddr::Multiaddr;
use sha2::Digest;
use sha2::Sha256;
use std::io::BufRead;
use std::io::BufReader;
use std::time::Duration;
use tofuri_block::BlockB;
use tofuri_core::*;
use tofuri_stake::StakeB;
use tofuri_transaction::TransactionB;
use tokio::time::Instant;
use tokio::time::Interval;
use tokio::time::MissedTickBehavior::Skip;
use tracing::error;
use tracing::info;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
use uint::construct_uint;
pub const GIT_HASH: &str = env!("GIT_HASH");
lazy_static! {
    pub static ref EMPTY_BLOCK_SIZE: usize = bincode::serialize(&BlockB::default()).unwrap().len();
    pub static ref TRANSACTION_SIZE: usize =
        bincode::serialize(&TransactionB::default()).unwrap().len();
    pub static ref STAKE_SIZE: usize = bincode::serialize(&StakeB::default()).unwrap().len();
    pub static ref MAINNET: Multiaddr = "/ip4/0.0.0.0/tcp/9333".parse().unwrap();
    pub static ref TESTNET: Multiaddr = "/ip4/0.0.0.0/tcp/9335".parse().unwrap();
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
pub fn block_timestamp() -> u32 {
    let timestamp = timestamp();
    timestamp - (timestamp % BLOCK_TIME)
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
        string.push_str(&format!(
            "{} {}{}",
            num,
            str,
            if num == 1 { "" } else { "s" }
        ));
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
    let mut interval = tokio::time::interval_at(start, duration);
    interval.set_missed_tick_behavior(Skip);
    interval
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
pub fn io_reload_filter(reload_handle: reload::Handle<EnvFilter, Registry>) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(std::io::stdin());
        let mut line = String::new();
        loop {
            _ = reader.read_line(&mut line);
            let filter = EnvFilter::new(line.trim());
            info!(?filter, "Reload");
            if let Err(e) = reload_handle.modify(|x| *x = filter) {
                error!(?e)
            }
            line.clear();
        }
    });
}
pub fn validate_block_timestamp(timestamp: u32, previous_timestamp: u32) -> bool {
    !(timestamp.saturating_sub(previous_timestamp) == 0 || timestamp % BLOCK_TIME != 0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_block_size_limit() {
        assert_eq!(
            BLOCK_SIZE_LIMIT,
            *EMPTY_BLOCK_SIZE + *TRANSACTION_SIZE * 600
        );
    }
}
