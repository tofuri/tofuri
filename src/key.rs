use crate::{constants::PREFIX_ADDRESS_KEY, types, util::hash};
use std::error::Error;
fn checksum(secret_key: &types::SecretKeyBytes) -> types::Checksum {
    hash(secret_key).get(1..5).unwrap().try_into().unwrap()
}
pub fn encode(secret_key: &ed25519_dalek::SecretKey) -> String {
    [
        PREFIX_ADDRESS_KEY,
        &hex::encode(secret_key),
        &hex::encode(checksum(secret_key.as_bytes())),
    ]
    .concat()
}
pub fn decode(secret_key: &str) -> Result<types::SecretKeyBytes, Box<dyn Error>> {
    let decoded = hex::decode(secret_key.replacen(PREFIX_ADDRESS_KEY, "", 1))?;
    println!("{:?}", decoded);
    let secret_key: types::SecretKeyBytes =
        decoded.get(0..32).ok_or("Invalid key")?.try_into().unwrap();
    if checksum(&secret_key) == decoded.get(32..).ok_or("Invalid checksum")? {
        Ok(secret_key)
    } else {
        Err("checksum mismatch".into())
    }
}
