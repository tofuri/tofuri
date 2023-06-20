use sha2::Digest;
use sha2::Sha256;
pub const PREFIX_PUBLIC: &str = "0x";
pub const PREFIX_SECRET: &str = "SECRETx";
pub fn checksum(bytes: &[u8]) -> [u8; 4] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let mut checksum = [0; 4];
    checksum.copy_from_slice(&hash[..4]);
    checksum
}
pub mod public {
    use super::*;
    #[derive(Debug)]
    pub enum Error {
        Hex(hex::FromHexError),
        Length,
        Checksum,
    }
    pub fn encode(address: &[u8; 20]) -> String {
        [
            PREFIX_PUBLIC,
            &hex::encode(address),
            &hex::encode(checksum(address)),
        ]
        .concat()
    }
    pub fn decode(str: &str) -> Result<[u8; 20], Error> {
        let decoded = hex::decode(str.replacen(PREFIX_PUBLIC, "", 1)).map_err(Error::Hex)?;
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
}
pub mod secret {
    use super::*;
    #[derive(Debug)]
    pub enum Error {
        Hex(hex::FromHexError),
        Length,
        Checksum,
    }
    pub fn encode(secret_key: &[u8; 32]) -> String {
        [
            PREFIX_SECRET,
            &hex::encode(secret_key),
            &hex::encode(checksum(secret_key)),
        ]
        .concat()
    }
    pub fn decode(str: &str) -> Result<[u8; 32], Error> {
        let decoded = hex::decode(str.replacen(PREFIX_SECRET, "", 1)).map_err(Error::Hex)?;
        let secret_key_bytes: [u8; 32] =
            decoded.get(0..32).ok_or(Error::Length)?.try_into().unwrap();
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
                decode("SECRETx000000000000000000000000000000000000000000000000000000000000000066687aad").unwrap(),
                [0; 32]
            );
        }
    }
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
