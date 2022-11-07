use ed25519::signature::Signer;
use ed25519_dalek::{Keypair, PublicKey, SecretKey};
use pea_address as address;
use pea_core::types;
use rand::rngs::OsRng;
pub struct Key {
    keypair: Keypair,
}
impl Key {
    pub fn generate() -> Key {
        let mut csprng = OsRng {};
        let keypair = Keypair::generate(&mut csprng);
        Key { keypair }
    }
    pub fn from_secret_key_bytes(secret_key_bytes: &[u8; 32]) -> Key {
        let secret_key = SecretKey::from_bytes(secret_key_bytes).unwrap();
        let public_key: PublicKey = (&secret_key).into();
        let keypair = Keypair {
            secret: secret_key,
            public: public_key,
        };
        Key { keypair }
    }
    pub fn public_key_bytes(&self) -> types::PublicKeyBytes {
        self.keypair.public.to_bytes()
    }
    pub fn secret_key_bytes(&self) -> types::SecretKeyBytes {
        self.keypair.secret.to_bytes()
    }
    pub fn public(&self) -> String {
        address::public::encode(&self.public_key_bytes())
    }
    pub fn secret(&self) -> String {
        address::secret::encode(&self.secret_key_bytes())
    }
    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.keypair.sign(msg).to_bytes()
    }
    pub fn verify(public_key_bytes: &types::PublicKeyBytes, message: &[u8], signature_bytes: &types::SignatureBytes) -> Result<(), Box<dyn std::error::Error>> {
        let public_key = types::PublicKey::from_bytes(public_key_bytes)?;
        let signature = types::Signature::from_bytes(signature_bytes)?;
        Ok(public_key.verify_strict(message, &signature)?)
    }
}
