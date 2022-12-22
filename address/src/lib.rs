pub mod address {
    use pea_core::{constants::PREFIX_ADDRESS, types, util};
    use std::error::Error;
    pub fn checksum(address: &types::AddressBytes) -> types::Checksum {
        util::hash(address).get(0..4).unwrap().try_into().unwrap()
    }
    pub fn encode(address: &types::AddressBytes) -> String {
        [PREFIX_ADDRESS, &hex::encode(address), &hex::encode(checksum(address))].concat()
    }
    pub fn decode(str: &str) -> Result<types::AddressBytes, Box<dyn Error>> {
        let decoded = hex::decode(str.replacen(PREFIX_ADDRESS, "", 1))?;
        let address: types::AddressBytes = decoded.get(0..20).ok_or("Invalid address")?.try_into().unwrap();
        if checksum(&address) == decoded.get(20..).ok_or("Invalid address checksum")? {
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
            assert_eq!("0x0000000000000000000000000000000000000000607b1aff", encode(&[0; 20]));
        }
        #[test]
        fn test_decode() {
            assert_eq!([0; 20], decode("0x0000000000000000000000000000000000000000607b1aff").unwrap());
        }
        #[test]
        fn test_cecksum() {
            assert_eq!(vec![96, 123, 26, 255], checksum(&[0; 20]));
        }
    }
}
pub mod public {
    use pea_core::{constants::PREFIX_ADDRESS, types, util};
    use std::error::Error;
    pub fn checksum(public_key: &types::PublicKeyBytes) -> types::Checksum {
        util::hash(public_key).get(0..4).unwrap().try_into().unwrap()
    }
    pub fn encode(public_key: &types::PublicKeyBytes) -> String {
        [PREFIX_ADDRESS, &hex::encode(public_key), &hex::encode(checksum(public_key))].concat()
    }
    pub fn decode(str: &str) -> Result<types::PublicKeyBytes, Box<dyn Error>> {
        let decoded = hex::decode(str.replacen(PREFIX_ADDRESS, "", 1))?;
        let public_key: types::PublicKeyBytes = decoded.get(0..32).ok_or("Invalid public key")?.try_into().unwrap();
        if checksum(&public_key) == decoded.get(32..).ok_or("Invalid public key checksum")? {
            Ok(public_key)
        } else {
            Err("checksum mismatch".into())
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn test_encode() {
            assert_eq!("0x00000000000000000000000000000000000000000000000000000000000000002ada83c1", encode(&[0; 32]));
        }
        #[test]
        fn test_decode() {
            assert_eq!(
                [0; 32],
                decode("0x00000000000000000000000000000000000000000000000000000000000000002ada83c1").unwrap()
            );
        }
        #[test]
        fn test_cecksum() {
            assert_eq!(vec![0x2a, 0xda, 0x83, 0xc1], checksum(&[0; 32]));
        }
    }
}
pub mod secret {
    use pea_core::{constants::PREFIX_ADDRESS_KEY, types, util};
    use std::error::Error;
    pub fn checksum(secret_key: &types::SecretKeyBytes) -> types::Checksum {
        util::hash(secret_key).get(4..8).unwrap().try_into().unwrap()
    }
    pub fn encode(secret_key: &types::SecretKeyBytes) -> String {
        [PREFIX_ADDRESS_KEY, &hex::encode(secret_key), &hex::encode(checksum(secret_key))].concat()
    }
    pub fn decode(str: &str) -> Result<types::SecretKeyBytes, Box<dyn Error>> {
        let decoded = hex::decode(str.replacen(PREFIX_ADDRESS_KEY, "", 1))?;
        let secret_key: types::SecretKeyBytes = decoded.get(0..32).ok_or("Invalid secret key")?.try_into().unwrap();
        if checksum(&secret_key) == decoded.get(32..).ok_or("Invalid secret key checksum")? {
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
                "SECRETx0000000000000000000000000000000000000000000000000000000000000000819a5372",
                encode(&[0; 32])
            );
        }
        #[test]
        fn test_decode() {
            assert_eq!(
                [0; 32],
                decode("SECRETx0000000000000000000000000000000000000000000000000000000000000000819a5372").unwrap()
            );
        }
        #[test]
        fn test_cecksum() {
            assert_eq!(vec![129, 154, 83, 114], checksum(&[0; 32]));
        }
    }
}
