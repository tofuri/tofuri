use crate::{constants::PREFIX_ADDRESS, types, util};
use std::error::Error;
fn checksum(public_key: &types::PublicKeyBytes) -> types::Checksum {
    util::hash(public_key)
        .get(0..4)
        .unwrap()
        .try_into()
        .unwrap()
}
pub fn encode(public_key: &types::PublicKeyBytes) -> String {
    [
        PREFIX_ADDRESS,
        &hex::encode(public_key),
        &hex::encode(checksum(public_key)),
    ]
    .concat()
}
pub fn decode(address: &str) -> Result<types::PublicKeyBytes, Box<dyn Error>> {
    let decoded = hex::decode(address.replacen(PREFIX_ADDRESS, "", 1))?;
    let public_key: types::PublicKeyBytes = decoded
        .get(0..32)
        .ok_or("Invalid address")?
        .try_into()
        .unwrap();
    if checksum(&public_key) == decoded.get(32..).ok_or("Invalid checksum")? {
        Ok(public_key)
    } else {
        Err("checksum mismatch".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[test]
    fn test_encode() {
        assert_eq!(
            "0x00000000000000000000000000000000000000000000000000000000000000002ada83c1",
            encode(&[0; 32])
        );
    }
    #[test]
    fn test_decode() {
        assert_eq!(
            [0; 32],
            decode("0x00000000000000000000000000000000000000000000000000000000000000002ada83c1")
                .unwrap()
        );
    }
    #[test]
    fn test_cecksum() {
        assert_eq!(vec![0x2a, 0xda, 0x83, 0xc1], checksum(&[0; 32]));
    }
    #[bench]
    fn bench_cecksum(b: &mut Bencher) {
        b.iter(|| checksum(&[0; 32]));
    }
}
