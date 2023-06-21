use crate::checksum;
use crate::Error;
pub const PREFIX: &str = "0x";
pub fn encode(address: &[u8; 20]) -> String {
    [
        PREFIX,
        &hex::encode(address),
        &hex::encode(checksum(address)),
    ]
    .concat()
}
pub fn decode(str: &str) -> Result<[u8; 20], Error> {
    let decoded = hex::decode(str.replacen(PREFIX, "", 1)).map_err(Error::Hex)?;
    let address_bytes: [u8; 20] = decoded.get(0..20).ok_or(Error::Length)?.try_into().unwrap();
    if checksum(&address_bytes) == decoded.get(20..).ok_or(Error::Length)? {
        Ok(address_bytes)
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
            "0x0000000000000000000000000000000000000000de47c9b2",
            encode(&[0; 20])
        );
    }
    #[test]
    fn test_decode() {
        assert_eq!(
            [0; 20],
            decode("0x0000000000000000000000000000000000000000de47c9b2").unwrap()
        );
    }
}
