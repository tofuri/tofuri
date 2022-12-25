use crate::types;
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
pub fn timestamp() -> u32 {
    chrono::offset::Utc::now().timestamp() as u32
}
pub fn address(public_key_bytes: &types::PublicKeyBytes) -> types::AddressBytes {
    let mut hasher = blake3::Hasher::new();
    hasher.update(public_key_bytes);
    let mut output = [0; 20];
    let mut output_reader = hasher.finalize_xof();
    output_reader.fill(&mut output);
    output
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
