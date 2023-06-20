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
pub fn timestamp() -> u32 {
    chrono::offset::Utc::now().timestamp() as u32
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
