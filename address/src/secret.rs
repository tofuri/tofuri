use crate::checksum;
use crate::Error;
pub const PREFIX: &str = "SECRETx";
pub fn encode(secret_key: &[u8; 32]) -> String {
    [
        PREFIX,
        &hex::encode(secret_key),
        &hex::encode(checksum(secret_key)),
    ]
    .concat()
}
pub fn decode(str: &str) -> Result<[u8; 32], Error> {
    let decoded = hex::decode(str.replacen(PREFIX, "", 1)).map_err(Error::Hex)?;
    let secret_key_bytes: [u8; 32] = decoded.get(0..32).ok_or(Error::Length)?.try_into().unwrap();
    if checksum(&secret_key_bytes) == decoded.get(32..).ok_or(Error::Length)? {
        Ok(secret_key_bytes)
    } else {
        Err(Error::Checksum)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_encode() {
        assert_eq!(
            encode(&[0; 32]),
            "SECRETx000000000000000000000000000000000000000000000000000000000000000066687aad"
        );
    }
    #[test]
    fn test_decode() {
        assert_eq!(
            decode(
                "SECRETx000000000000000000000000000000000000000000000000000000000000000066687aad"
            )
            .unwrap(),
            [0; 32]
        );
    }
}
