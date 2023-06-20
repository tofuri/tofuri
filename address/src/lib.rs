pub mod public;
pub mod secret;
use sha2::Digest;
use sha2::Sha256;
#[derive(Debug)]
pub enum Error {
    Hex(hex::FromHexError),
    Length,
    Checksum,
}
pub fn checksum(bytes: &[u8]) -> [u8; 4] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let mut checksum = [0; 4];
    checksum.copy_from_slice(&hash[..4]);
    checksum
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cecksum() {
        assert_eq!(checksum(&[0; 32]), [102, 104, 122, 173]);
        assert_eq!(checksum(&[0; 33]), [127, 156, 158, 49]);
    }
}
