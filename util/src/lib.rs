use colored::*;
use lazy_static::lazy_static;
use multiaddr::Multiaddr;
use std::io::BufRead;
use std::io::BufReader;
use tofuri_block::Block;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
use tracing::error;
use tracing::info;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
pub const BLOCK_SIZE_LIMIT: usize = 57797;
pub const GIT_HASH: &str = env!("GIT_HASH");
pub const MAINNET_PORT: u16 = 2020;
pub const TESTNET_PORT: u16 = 3030;
pub const BLOCK_TIME: u32 = 60;
lazy_static! {
    pub static ref EMPTY_BLOCK_SIZE: usize = bincode::serialize(&Block::default()).unwrap().len();
    pub static ref TRANSACTION_SIZE: usize =
        bincode::serialize(&Transaction::default()).unwrap().len();
    pub static ref STAKE_SIZE: usize = bincode::serialize(&Stake::default()).unwrap().len();
    pub static ref MAINNET: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", MAINNET_PORT)
        .parse()
        .unwrap();
    pub static ref TESTNET: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", TESTNET_PORT)
        .parse()
        .unwrap();
}
pub fn penalty(index: usize) -> u128 {
    if index == 0 {
        return 0;
    }
    10_u128.pow(18) * 2_u128.pow(index as u32 - 1)
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
