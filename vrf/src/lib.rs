use curve25519_dalek::constants::RISTRETTO_BASEPOINT_TABLE;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use digest::generic_array::typenum::U32;
use digest::generic_array::typenum::U64;
use digest::generic_array::GenericArray;
use digest::Digest;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone)]
pub struct Proof {
    gamma: [u8; 32],
    c: [u8; 32],
    s: [u8; 32],
}
impl Proof {
    pub fn to_bytes(&self) -> [u8; 96] {
        let mut bytes = [0; 96];
        for i in 0..32 {
            bytes[i] = self.gamma[i];
            bytes[32 + i] = self.c[i];
            bytes[64 + i] = self.s[i];
        }
        bytes
    }
    pub fn from_bytes(input: &[u8; 96]) -> Proof {
        let mut gamma = [0; 32];
        let mut c = [0; 32];
        let mut s = [0; 32];
        gamma.copy_from_slice(&input[0..32]);
        c.copy_from_slice(&input[32..64]);
        s.copy_from_slice(&input[64..96]);
        Proof { gamma, c, s }
    }
    pub fn hash<D>(&self) -> GenericArray<u8, <D as Digest>::OutputSize>
    where
        D: Digest + Default,
    {
        let mut hasher = D::default();
        hasher.update(self.gamma);
        hasher.finalize()
    }
    pub fn new<D512, D256>(secret: &Scalar, alpha: &[u8]) -> Proof
    where
        D512: Digest<OutputSize = U64> + Default,
        D256: Digest<OutputSize = U32> + Default,
    {
        let h = RistrettoPoint::hash_from_bytes::<D512>(alpha);
        let p = &RISTRETTO_BASEPOINT_TABLE * secret;
        let gamma = h * secret;
        let k: Scalar = Scalar::random(&mut OsRng);
        let mut hasher = D256::default();
        hasher.update(
            [
                compress_to_bytes(h),
                compress_to_bytes(p),
                compress_to_bytes(gamma),
                compress_to_bytes(&RISTRETTO_BASEPOINT_TABLE * &k),
                compress_to_bytes(h * k),
            ]
            .concat(),
        );
        let c = hasher.finalize().into();
        let c_scalar = Scalar::from_bytes_mod_order(c);
        let s = k - c_scalar * secret;
        Proof {
            gamma: compress_to_bytes(gamma),
            c,
            s: s.to_bytes(),
        }
    }
    pub fn verify<D512, D256, D>(&self, public: &[u8], alpha: &[u8], beta: &[u8]) -> bool
    where
        D512: Digest<OutputSize = U64> + Default,
        D256: Digest<OutputSize = U32> + Default,
        D: Digest + Default,
    {
        let y = from_slice_decompress(public).expect("valid key");
        let gamma = from_slice_decompress(&self.gamma);
        if gamma.is_none() {
            return false;
        }
        let gamma = gamma.unwrap();
        let mut hasher = D::default();
        hasher.update(compress_to_bytes(gamma));
        if beta != hasher.finalize().as_slice() {
            return false;
        }
        let s = Scalar::from_canonical_bytes(self.s);
        if s.is_none() {
            return false;
        }
        let s = s.unwrap();
        let c_scalar = Scalar::from_bytes_mod_order(self.c);
        let u = y * c_scalar + &RISTRETTO_BASEPOINT_TABLE * &s;
        let h = RistrettoPoint::hash_from_bytes::<D512>(alpha);
        let v = gamma * c_scalar + h * s;
        let mut hasher = D256::default();
        hasher.update(
            [
                compress_to_bytes(h),
                compress_to_bytes(y),
                compress_to_bytes(gamma),
                compress_to_bytes(u),
                compress_to_bytes(v),
            ]
            .concat(),
        );
        if self.c != hasher.finalize().as_slice() {
            return false;
        }
        true
    }
}
fn compress_to_bytes(p: RistrettoPoint) -> [u8; 32] {
    p.compress().to_bytes()
}
fn from_slice_decompress(bytes: &[u8]) -> Option<RistrettoPoint> {
    CompressedRistretto::from_slice(bytes).decompress()
}
pub fn validate_key(public: &[u8]) -> bool {
    from_slice_decompress(public).is_some()
}
#[cfg(test)]
mod tests {
    use super::*;
    use pea_key::Key;
    use sha3::Sha3_256;
    use sha3::Sha3_512;
    #[test]
    fn test_proof() {
        let key = Key::generate();
        let alpha = [];
        let proof_0 = Proof::new::<Sha3_512, Sha3_256>(&key.scalar, &alpha);
        let proof_1 = Proof::new::<Sha3_512, Sha3_256>(&key.scalar, &alpha);
        assert_eq!(proof_0.gamma, proof_1.gamma);
        assert_ne!(proof_0.c, proof_1.c);
        assert_ne!(proof_0.s, proof_1.s);
        let beta_0 = proof_0.hash::<Sha3_256>();
        let beta_1 = proof_1.hash::<Sha3_256>();
        assert_eq!(beta_0, beta_1);
    }
    #[test]
    fn test_verify() {
        let key = Key::generate();
        let alpha = [];
        let pi = Proof::new::<Sha3_512, Sha3_256>(&key.scalar, &alpha);
        let beta = pi.hash::<Sha3_256>();
        assert!(pi.verify::<Sha3_512, Sha3_256, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha, &beta));
    }
    #[test]
    fn test_verify_fake() {
        let key = Key::generate();
        let key_fake = Key::generate();
        let alpha = [0];
        let alpha_fake = [1];
        let proof = Proof::new::<Sha3_512, Sha3_256>(&key.scalar, &alpha);
        let beta = proof.hash::<Sha3_256>();
        let proof_fake = Proof::new::<Sha3_512, Sha3_256>(&key_fake.scalar, &alpha);
        let beta_fake_0 = proof_fake.hash::<Sha3_256>();
        let mut beta_fake_1 = beta.clone();
        beta_fake_1[0] += 0x01;
        assert!(!proof.verify::<Sha3_512, Sha3_256, Sha3_256>(key_fake.compressed_ristretto().as_bytes(), &alpha, &beta));
        assert!(!proof.verify::<Sha3_512, Sha3_256, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha_fake, &beta));
        assert!(!proof.verify::<Sha3_512, Sha3_256, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha, &beta_fake_0));
        assert!(!proof.verify::<Sha3_512, Sha3_256, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha, &beta_fake_1));
        assert!(!proof_fake.verify::<Sha3_512, Sha3_256, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha, &beta));
    }
    #[test]
    fn test_validate_key() {
        let key = Key::generate();
        assert!(validate_key(key.compressed_ristretto().as_bytes()));
    }
}
