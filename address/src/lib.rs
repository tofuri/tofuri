pub fn checksum(address: &[u8]) -> [u8; 4] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(address);
    let mut output = [0; 4];
    let mut output_reader = hasher.finalize_xof();
    output_reader.fill(&mut output);
    output
}
pub mod address {
    use super::*;
    use pea_core::{constants::PREFIX_ADDRESS, types};
    use std::error::Error;
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
    }
}
pub mod public {
    use super::*;
    use pea_core::{constants::PREFIX_ADDRESS, types};
    use std::error::Error;
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
    }
}
pub mod secret {
    use super::*;
    use pea_core::{constants::PREFIX_ADDRESS_KEY, types};
    use std::error::Error;
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
                "SECRETx00000000000000000000000000000000000000000000000000000000000000002ada83c1",
                encode(&[0; 32])
            );
        }
        #[test]
        fn test_decode() {
            assert_eq!(
                [0; 32],
                decode("SECRETx00000000000000000000000000000000000000000000000000000000000000002ada83c1").unwrap()
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cecksum() {
        assert_eq!(vec![0x2a, 0xda, 0x83, 0xc1], checksum(&[0; 32]));
    }
}
