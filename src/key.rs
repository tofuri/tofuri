use crate::{constants::PREFIX_ADDRESS_KEY, types, util};
use std::error::Error;
fn checksum(secret_key: &types::SecretKeyBytes) -> types::Checksum {
    util::hash(secret_key)
        .get(4..8)
        .unwrap()
        .try_into()
        .unwrap()
}
pub fn encode(secret_key: &types::SecretKeyBytes) -> String {
    [
        PREFIX_ADDRESS_KEY,
        &hex::encode(secret_key),
        &hex::encode(checksum(secret_key)),
    ]
    .concat()
}
pub fn decode(secret_key: &str) -> Result<types::SecretKeyBytes, Box<dyn Error>> {
    let decoded = hex::decode(secret_key.replacen(PREFIX_ADDRESS_KEY, "", 1))?;
    let secret_key: types::SecretKeyBytes =
        decoded.get(0..32).ok_or("Invalid key")?.try_into().unwrap();
    if checksum(&secret_key) == decoded.get(32..).ok_or("Invalid checksum")? {
        Ok(secret_key)
    } else {
        Err("checksum mismatch".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_encode() {
        assert_eq!(
            "Key0x0000000000000000000000000000000000000000000000000000000000000000819a5372",
            encode(&[0; 32])
        );
    }
    #[test]
    fn test_decode() {
        assert_eq!(
            [0; 32],
            decode("Key0x0000000000000000000000000000000000000000000000000000000000000000819a5372")
                .unwrap()
        );
    }
}
