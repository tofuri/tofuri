use crate::{constants::PREFIX_ADDRESS, types, util};
use std::error::Error;
fn checksum(decoded: &[u8]) -> [u8; 4] {
    util::hash(decoded).get(0..4).unwrap().try_into().unwrap()
}
pub fn encode(public_key: &types::PublicKey) -> String {
    [
        PREFIX_ADDRESS,
        &hex::encode(public_key),
        &hex::encode(checksum(public_key)),
    ]
    .concat()
}
pub fn decode(address: &str) -> Result<types::PublicKey, Box<dyn Error>> {
    let decoded = hex::decode(address.replacen(PREFIX_ADDRESS, "", 1))?;
    let public_key: types::PublicKey = decoded
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
    fn test_cecksum() {
        assert_eq!(vec![0x60, 0x7b, 0x1a, 0xff], checksum(&[0; 20]));
    }
    #[bench]
    fn bench_cecksum(b: &mut Bencher) {
        b.iter(|| checksum(&[0; 20]));
    }
}
