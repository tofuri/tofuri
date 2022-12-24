use curve25519_dalek::{constants::RISTRETTO_BASEPOINT_TABLE, ristretto::RistrettoPoint, scalar::Scalar};
use ed25519::signature::Signer;
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature};
use pea_core::{types, util};
use rand::rngs::OsRng;
#[derive(Debug)]
pub struct Key {
    pub scalar: Scalar,
}
impl Key {
    pub fn generate() -> Key {
        Key {
            scalar: Scalar::random(&mut OsRng),
        }
    }
    pub fn from_canonical_bytes(secret_key_bytes: types::SecretKeyBytes) -> Option<Key> {
        if let Some(scalar) = Scalar::from_canonical_bytes(secret_key_bytes) {
            Some(Key { scalar })
        } else {
            None
        }
    }
    pub fn from_bytes_mod_order(secret_key_bytes: types::SecretKeyBytes) -> Key {
        Key {
            scalar: Scalar::from_bytes_mod_order(secret_key_bytes),
        }
    }
    pub fn secret_key(&self) -> SecretKey {
        SecretKey::from_bytes(self.scalar.as_bytes()).unwrap()
    }
    pub fn secret_key_bytes(&self) -> types::SecretKeyBytes {
        self.secret_key().to_bytes()
    }
    pub fn public_key(&self) -> PublicKey {
        (&self.secret_key()).into()
    }
    pub fn public_key_bytes(&self) -> types::PublicKeyBytes {
        self.public_key().to_bytes()
    }
    pub fn address_bytes(&self) -> types::AddressBytes {
        util::address(&self.public_key_bytes())
    }
    pub fn ristretto_point(&self) -> RistrettoPoint {
        &RISTRETTO_BASEPOINT_TABLE * &self.scalar
    }
    pub fn keypair(&self) -> Keypair {
        Keypair {
            secret: self.secret_key(),
            public: self.public_key(),
        }
    }
    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.keypair().sign(msg).to_bytes()
    }
    pub fn verify(public_key_bytes: &types::PublicKeyBytes, message: &[u8], signature_bytes: &types::SignatureBytes) -> Result<(), Box<dyn std::error::Error>> {
        let public_key = PublicKey::from_bytes(public_key_bytes)?;
        let signature = Signature::from_bytes(signature_bytes)?;
        Ok(public_key.verify_strict(message, &signature)?)
    }
    pub fn subkey(&self, n: usize) -> Key {
        let mut vec = self.secret_key_bytes().to_vec();
        vec.append(&mut n.to_le_bytes().to_vec());
        let hash = blake3::hash(&vec).into();
        Key::from_bytes_mod_order(hash)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sign_verify() {
        let key = Key::generate();
        let message = [0; 128];
        let signature_bytes = key.sign(&message);
        assert!(Key::verify(&key.public_key_bytes(), &message, &signature_bytes).is_ok());
    }
}
