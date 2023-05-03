use sha2::Digest;
use sha2::Sha256;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use tofuri_core::*;
#[derive(Debug)]
pub enum Error {
    Hex(hex::FromHexError),
    InvalidAddress,
    InvalidAddressChecksum,
    AddressChecksumMismatch,
    InvalidSecretKey,
    InvalidSecretKeyChecksum,
    SecretKeyChecksumMismatch,
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Hex(err) => Some(err),
            _ => None,
        }
    }
}
pub fn checksum(bytes: &[u8]) -> [u8; 4] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let mut checksum = [0; 4];
    checksum.copy_from_slice(&hash[..4]);
    checksum
}
pub mod address {
    use super::*;
    pub fn encode(address: &AddressBytes) -> String {
        [
            PREFIX_ADDRESS,
            &hex::encode(address),
            &hex::encode(checksum(address)),
        ]
        .concat()
    }
    pub fn decode(str: &str) -> Result<AddressBytes, Error> {
        let decoded = hex::decode(str.replacen(PREFIX_ADDRESS, "", 1)).map_err(Error::Hex)?;
        let address_bytes: AddressBytes = decoded
            .get(0..20)
            .ok_or(Error::InvalidAddress)?
            .try_into()
            .unwrap();
        if checksum(&address_bytes) == decoded.get(20..).ok_or(Error::InvalidAddressChecksum)? {
            Ok(address_bytes)
        } else {
            Err(Error::AddressChecksumMismatch)
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
    pub fn encode(secret_key: &SecretKeyBytes) -> String {
        [
            PREFIX_SECRET_KEY,
            &hex::encode(secret_key),
            &hex::encode(checksum(secret_key)),
        ]
        .concat()
    }
    pub fn decode(str: &str) -> Result<SecretKeyBytes, Error> {
        let decoded = hex::decode(str.replacen(PREFIX_SECRET_KEY, "", 1)).map_err(Error::Hex)?;
        let secret_key_bytes: SecretKeyBytes = decoded
            .get(0..32)
            .ok_or(Error::InvalidSecretKey)?
            .try_into()
            .unwrap();
        if checksum(&secret_key_bytes)
            == decoded.get(32..).ok_or(Error::InvalidSecretKeyChecksum)?
        {
            Ok(secret_key_bytes)
        } else {
            Err(Error::SecretKeyChecksumMismatch)
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
