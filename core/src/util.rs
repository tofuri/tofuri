use crate::{
    constants::{COIN, MIN_STAKE_MULTIPLIER},
    types,
};
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
pub fn timestamp() -> u32 {
    chrono::offset::Utc::now().timestamp() as u32
}
pub fn hash(input: &[u8]) -> types::Hash {
    blake3::hash(input).into()
}
pub fn read_lines(path: impl AsRef<Path>) -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let buf = BufReader::new(file);
    Ok(buf.lines().map(|l| l.expect("Could not parse line")).collect())
}
pub fn reward(balance_staked: u128) -> u128 {
    ((2f64.powf((balance_staked as f64 / COIN as f64) / MIN_STAKE_MULTIPLIER as f64) - 1f64) * COIN as f64) as u128
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
#[cfg(test)]
mod tests {
    #[test]
    fn test_hash() {
        assert_eq!(
            blake3::hash(b"test").to_string(),
            "4878ca0425c739fa427f7eda20fe845f6b2e46ba5fe2a14df5b1e32f50603215".to_string()
        );
    }
}
