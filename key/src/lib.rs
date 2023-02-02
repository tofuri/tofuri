use pea_core::*;
use secp256k1::ecdsa::RecoverableSignature;
use secp256k1::ecdsa::RecoveryId;
use secp256k1::Message;
use secp256k1::PublicKey;
use secp256k1::SecretKey;
use secp256k1::SECP256K1;
use sha2::Digest;
use sha2::Sha256;
use std::error::Error;
#[cfg(feature = "vrf")]
use vrf::openssl::CipherSuite;
#[cfg(feature = "vrf")]
use vrf::openssl::ECVRF;
#[cfg(feature = "vrf")]
use vrf::VRF;
#[derive(Debug, Clone, Copy)]
pub struct Key {
    pub secret_key: SecretKey,
}
impl Key {
    pub fn generate() -> Key {
        Key {
            secret_key: SecretKey::new(&mut rand::thread_rng()),
        }
    }
    pub fn from_slice(secret_key_bytes: &SecretKeyBytes) -> Key {
        Key {
            secret_key: SecretKey::from_slice(secret_key_bytes).expect("32 bytes, within curve order"),
        }
    }
    pub fn secret_key_bytes(&self) -> SecretKeyBytes {
        self.secret_key.secret_bytes()
    }
    pub fn public_key(&self) -> PublicKey {
        self.secret_key.public_key(SECP256K1)
    }
    pub fn public_key_bytes(&self) -> PublicKeyBytes {
        self.public_key().serialize()
    }
    pub fn address_bytes(&self) -> AddressBytes {
        Key::address(&self.public_key_bytes())
    }
    pub fn address(public_key_bytes: &PublicKeyBytes) -> AddressBytes {
        let mut hasher = Sha256::new();
        hasher.update(public_key_bytes);
        let hash = hasher.finalize();
        let mut address = [0; 20];
        address.copy_from_slice(&hash[..20]);
        address
    }
    pub fn sign(&self, hash: &Hash) -> Result<SignatureBytes, Box<dyn Error>> {
        let message = Message::from_slice(hash)?;
        Ok(loop {
            let signature = SECP256K1.sign_ecdsa_recoverable_with_noncedata(&message, &self.secret_key, &rand::random());
            let (recovery_id, signature_bytes) = signature.serialize_compact();
            if recovery_id.to_i32() == RECOVERY_ID {
                break signature_bytes;
            }
        })
    }
    pub fn recover(hash: &Hash, signature_bytes: &SignatureBytes) -> Result<PublicKeyBytes, Box<dyn Error>> {
        let message = Message::from_slice(hash)?;
        let signature = RecoverableSignature::from_compact(signature_bytes, RecoveryId::from_i32(RECOVERY_ID).unwrap())?;
        let public_key_bytes: PublicKeyBytes = SECP256K1.recover_ecdsa(&message, &signature)?.serialize();
        Ok(public_key_bytes)
    }
    #[cfg(feature = "vrf")]
    pub fn vrf_prove(&self, alpha: &[u8]) -> Option<Pi> {
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let pi = vrf.prove(&self.secret_key_bytes(), alpha);
        if pi.is_err() {
            return None;
        }
        Some(pi.unwrap().try_into().unwrap())
    }
    #[cfg(feature = "vrf")]
    pub fn vrf_proof_to_hash(pi: &[u8]) -> Option<Beta> {
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let beta = vrf.proof_to_hash(pi);
        if beta.is_err() {
            return None;
        }
        Some(beta.unwrap().try_into().unwrap())
    }
    #[cfg(feature = "vrf")]
    pub fn vrf_verify(y: &[u8], pi: &[u8], alpha: &[u8]) -> Option<Beta> {
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let beta = vrf.verify(y, pi, alpha);
        if beta.is_err() {
            return None;
        }
        Some(beta.unwrap().try_into().unwrap())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_address() {
        assert_eq!(
            Key::address(&[0; 33]),
            [127, 156, 158, 49, 172, 130, 86, 202, 47, 37, 133, 131, 223, 38, 45, 188, 125, 111, 104, 242]
        );
    }
    #[test]
    fn test_sign_verify() {
        let key = Key::generate();
        let hash = [0; 32];
        let signature_bytes = key.sign(&hash).unwrap();
        assert_eq!(key.public_key_bytes(), Key::recover(&hash, &signature_bytes).unwrap());
    }
    #[test]
    #[cfg(feature = "vrf")]
    fn test_vrf_public_key() {
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let key = Key::generate();
        let public_key = vrf.derive_public_key(&key.secret_key_bytes()).unwrap();
        assert_eq!(key.public_key_bytes().to_vec(), public_key);
    }
    #[test]
    #[cfg(feature = "vrf")]
    fn test_vrf_prove_verify() {
        let key = Key::generate();
        let alpha: [u8; 32] = rand::random();
        let pi = key.vrf_prove(&alpha).unwrap();
        let beta = Key::vrf_verify(&key.public_key_bytes(), &pi, &alpha);
        assert!(beta.unwrap() == Key::vrf_proof_to_hash(&pi).unwrap());
    }
}
