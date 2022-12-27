use sha2::{Digest, Sha256};
pub mod address {
    use super::*;
    use pea_core::{constants::PREFIX_ADDRESS, types};
    use std::error::Error;
    pub fn checksum(bytes: &[u8]) -> [u8; 4] {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        let mut checksum = [0; 4];
        checksum.copy_from_slice(&hash[..4]);
        checksum
    }
    pub fn encode(address: &types::AddressBytes) -> String {
        [PREFIX_ADDRESS, &hex::encode(address), &hex::encode(checksum(address))].concat()
    }
    pub fn decode(str: &str) -> Result<types::AddressBytes, Box<dyn Error>> {
        let decoded = hex::decode(str.replacen(PREFIX_ADDRESS, "", 1))?;
        let address: types::AddressBytes = decoded.get(0..20).ok_or("invalid address")?.try_into().unwrap();
        if checksum(&address) == decoded.get(20..).ok_or("invalid address checksum")? {
            Ok(address)
        } else {
            Err("checksum mismatch".into())
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn test_encode() {
            assert_eq!("0x0000000000000000000000000000000000000000de47c9b2", encode(&[0; 20]));
        }
        #[test]
        fn test_decode() {
            assert_eq!([0; 20], decode("0x0000000000000000000000000000000000000000de47c9b2").unwrap());
        }
    }
}
pub mod secret {
    use super::*;
    use pea_core::{constants::PREFIX_ADDRESS_KEY, types};
    use std::error::Error;
    pub fn checksum(bytes: &[u8]) -> [u8; 4] {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        let mut checksum = [0; 4];
        checksum.copy_from_slice(&hash[4..8]);
        checksum
    }
    pub fn encode(secret_key: &types::SecretKeyBytes) -> String {
        [PREFIX_ADDRESS_KEY, &hex::encode(secret_key), &hex::encode(checksum(secret_key))].concat()
    }
    pub fn decode(str: &str) -> Result<types::SecretKeyBytes, Box<dyn Error>> {
        let decoded = hex::decode(str.replacen(PREFIX_ADDRESS_KEY, "", 1))?;
        let secret_key: types::SecretKeyBytes = decoded.get(0..32).ok_or("invalid secret key")?.try_into().unwrap();
        if checksum(&secret_key) == decoded.get(32..).ok_or("invalid secret key checksum")? {
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
                "SECRETx0000000000000000000000000000000000000000000000000000000000000000f862bd77",
                encode(&[0; 32])
            );
        }
        #[test]
        fn test_decode() {
            assert_eq!(
                [0; 32],
                decode("SECRETx0000000000000000000000000000000000000000000000000000000000000000f862bd77").unwrap()
            );
        }
    }
}
