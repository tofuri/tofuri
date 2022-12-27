use pea_core::{types, util};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    rand, Message, PublicKey, SecretKey, SECP256K1,
};
use sha2::{Digest, Sha256};
use std::error::Error;
use vrf::openssl::{CipherSuite, ECVRF};
use vrf::VRF;
const RECOVERY_ID: i32 = 0;
#[derive(Debug)]
pub struct Key {
    pub secret_key: SecretKey,
}
impl Key {
    pub fn generate() -> Key {
        Key {
            secret_key: SecretKey::new(&mut rand::thread_rng()),
        }
    }
    pub fn from_slice(secret_key_bytes: &types::SecretKeyBytes) -> Key {
        Key {
            secret_key: SecretKey::from_slice(secret_key_bytes).expect("32 bytes, within curve order"),
        }
    }
    pub fn secret_key_bytes(&self) -> types::SecretKeyBytes {
        self.secret_key.secret_bytes()
    }
    pub fn public_key(&self) -> PublicKey {
        self.secret_key.public_key(SECP256K1)
    }
    pub fn public_key_bytes(&self) -> types::PublicKeyBytes {
        self.public_key().serialize()
    }
    pub fn address_bytes(&self) -> types::AddressBytes {
        util::address(&self.public_key_bytes())
    }
    pub fn sign(&self, hash: &types::Hash) -> Result<types::SignatureBytes, Box<dyn Error>> {
        let message = Message::from_slice(hash)?;
        Ok(loop {
            let signature = SECP256K1.sign_ecdsa_recoverable_with_noncedata(&message, &self.secret_key, &rand::random());
            let (recovery_id, signature_bytes) = signature.serialize_compact();
            if recovery_id.to_i32() == RECOVERY_ID {
                break signature_bytes;
            }
        })
    }
    pub fn recover(hash: &types::Hash, signature_bytes: &types::SignatureBytes) -> Result<types::PublicKeyBytes, Box<dyn Error>> {
        let message = Message::from_slice(hash)?;
        let signature = RecoverableSignature::from_compact(signature_bytes, RecoveryId::from_i32(RECOVERY_ID).unwrap())?;
        let public_key_bytes: types::PublicKeyBytes = SECP256K1.recover_ecdsa(&message, &signature)?.serialize();
        Ok(public_key_bytes)
    }
    pub fn vrf_prove(&self, alpha: &[u8]) -> Option<[u8; 81]> {
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let pi = vrf.prove(&self.secret_key_bytes(), alpha);
        if let Err(_) = pi {
            return None;
        }
        Some(pi.unwrap().try_into().unwrap())
    }
    pub fn vrf_proof_to_hash(pi: &[u8]) -> Option<[u8; 32]> {
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let beta = vrf.proof_to_hash(pi);
        if let Err(_) = beta {
            return None;
        }
        Some(beta.unwrap().try_into().unwrap())
    }
    pub fn vrf_verify(y: &[u8], pi: &[u8], alpha: &[u8]) -> Option<[u8; 32]> {
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let beta = vrf.verify(y, pi, alpha);
        if let Err(_) = beta {
            return None;
        }
        Some(beta.unwrap().try_into().unwrap())
    }
    pub fn subkey(&self, n: usize) -> Key {
        let mut vec = self.secret_key_bytes().to_vec();
        vec.append(&mut n.to_le_bytes().to_vec());
        let mut hasher = Sha256::new();
        hasher.update(&vec);
        Key::from_slice(&hasher.finalize().into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sign_verify() {
        let key = Key::generate();
        let hash = [0; 32];
        let signature_bytes = key.sign(&hash).unwrap();
        assert_eq!(key.public_key_bytes(), Key::recover(&hash, &signature_bytes).unwrap());
    }
    #[test]
    fn test_vrf_public_key() {
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let key = Key::generate();
        let public_key = vrf.derive_public_key(&key.secret_key_bytes()).unwrap();
        assert_eq!(key.public_key_bytes().to_vec(), public_key);
    }
    #[test]
    fn test_vrf_prove_verify() {
        let key = Key::generate();
        let alpha: [u8; 32] = rand::random();
        let pi = key.vrf_prove(&alpha).unwrap();
        let beta = Key::vrf_verify(&key.public_key_bytes(), &pi, &alpha);
        assert!(beta.unwrap() == Key::vrf_proof_to_hash(&pi).unwrap());
    }
}
