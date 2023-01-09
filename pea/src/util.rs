use pea_core::*;
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
construct_uint! {
    pub struct U256(4);
}
pub fn u256(hash: &Hash) -> U256 {
    U256::from_big_endian(hash)
}
pub fn u256_modulo(hash: &Hash, modulo: u128) -> u128 {
    (u256(hash) % modulo).as_u128()
}
pub fn hash_n(hash: &Hash, n: u128) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(hash);
    hasher.update(n.to_be_bytes());
    hasher.finalize().into()
}
pub fn random(beta: &Beta, n: u128, modulo: u128) -> u128 {
    u256_modulo(&hash_n(beta, n), modulo)
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
pub fn read_lines(path: impl AsRef<Path>) -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let buf = BufReader::new(file);
    Ok(buf.lines().map(|l| l.expect("Could not parse line")).collect())
}
pub fn micros_per_tick(tps: f64) -> u64 {
    let secs = 1_f64 / tps;
    (secs * 1_000_000_f64) as u64
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
