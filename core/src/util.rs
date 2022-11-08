use crate::{
    constants::{DECIMAL_PRECISION, MIN_STAKE_MULTIPLIER},
    types,
};
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
pub fn timestamp() -> u32 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u32
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
    ((2f64.powf((balance_staked as f64 / DECIMAL_PRECISION as f64) / MIN_STAKE_MULTIPLIER as f64) - 1f64) * DECIMAL_PRECISION as f64) as u128
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[test]
    fn test_hash() {
        assert_eq!(blake3::hash(b"test").to_string(), "4878ca0425c739fa427f7eda20fe845f6b2e46ba5fe2a14df5b1e32f50603215".to_string());
    }
    #[bench]
    fn bench_hash(b: &mut Bencher) {
        b.iter(|| hash(b"test"));
    }
}
