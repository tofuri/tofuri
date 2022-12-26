use pea_core::{types, util};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    rand, Message, PublicKey, SecretKey, SECP256K1,
};
use std::error::Error;
#[derive(Debug)]
pub struct Key {
    pub secret_key: SecretKey,
}
impl Key {
    pub fn generate() -> Key {
        let secret_key = SecretKey::new(&mut rand::thread_rng());
        Key { secret_key }
    }
    pub fn from_slice(secret_key_bytes: &types::SecretKeyBytes) -> Key {
        let secret_key = SecretKey::from_slice(secret_key_bytes).expect("32 bytes, within curve order");
        Key { secret_key }
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
    pub fn sign(&self, hash: &types::Hash) -> Result<(types::RecoveryId, types::SignatureBytes), Box<dyn Error>> {
        let message = Message::from_slice(hash)?;
        let signature = SECP256K1.sign_ecdsa_recoverable(&message, &self.secret_key);
        let (recovery_id, signature_bytes) = signature.serialize_compact();
        Ok((recovery_id.to_i32() as u8, signature_bytes))
    }
    pub fn recover(hash: &types::Hash, signature_bytes: &types::SignatureBytes, recovery_id: types::RecoveryId) -> Result<types::AddressBytes, Box<dyn Error>> {
        let recovery_id = RecoveryId::from_i32(recovery_id as i32)?;
        let message = Message::from_slice(hash)?;
        let signature = RecoverableSignature::from_compact(signature_bytes, recovery_id)?;
        let public_key_bytes: types::PublicKeyBytes = SECP256K1.recover_ecdsa(&message, &signature)?.serialize();
        Ok(util::address(&public_key_bytes))
    }
    pub fn subkey(&self, n: usize) -> Key {
        let mut vec = self.secret_key_bytes().to_vec();
        vec.append(&mut n.to_le_bytes().to_vec());
        let hash = blake3::hash(&vec);
        Key::from_slice(hash.as_bytes())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sign_verify() {
        let key = Key::generate();
        let hash = [0; 32];
        let (recovery_id, signature_bytes) = key.sign(&hash).unwrap();
        assert_eq!(key.address_bytes(), Key::recover(&hash, &signature_bytes, recovery_id).unwrap());
    }
}
