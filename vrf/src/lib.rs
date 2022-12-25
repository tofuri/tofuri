use curve25519_dalek::constants::RISTRETTO_BASEPOINT_TABLE;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use digest::generic_array::typenum::U32;
use digest::generic_array::typenum::U64;
use digest::Digest;
use rand_core::OsRng;
#[derive(Debug, PartialEq, Eq)]
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
}
fn to_bytes(p: RistrettoPoint) -> [u8; 32] {
    p.compress().to_bytes()
}
fn from_bytes(bytes: &[u8]) -> Option<RistrettoPoint> {
    CompressedRistretto::from_slice(bytes).decompress()
}
pub fn validate_key(public: &[u8]) -> bool {
    from_bytes(public).is_some()
}
pub fn prove<A, B>(alpha: &[u8], secret: &Scalar) -> ([u8; 32], Proof)
where
    A: Digest<OutputSize = U64> + Default,
    B: Digest<OutputSize = U32> + Default,
{
    let h = RistrettoPoint::hash_from_bytes::<A>(alpha);
    let p = &RISTRETTO_BASEPOINT_TABLE * secret;
    let gamma = h * secret;
    let k: Scalar = Scalar::random(&mut OsRng);
    let mut hasher = B::default();
    hasher.update(
        [
            to_bytes(h),
            to_bytes(p),
            to_bytes(gamma),
            to_bytes(&RISTRETTO_BASEPOINT_TABLE * &k),
            to_bytes(h * k),
        ]
        .concat(),
    );
    let c = hasher.finalize().into();
    let c_scalar = Scalar::from_bytes_mod_order(c);
    let s = k - c_scalar * secret;
    let mut hasher = B::default();
    hasher.update(to_bytes(gamma));
    let beta = hasher.finalize().into();
    (
        beta,
        Proof {
            gamma: gamma.compress().to_bytes(),
            c,
            s: s.to_bytes(),
        },
    )
}
pub fn verify<A, B>(public: &[u8], alpha: &[u8], beta: &[u8; 32], pi: &Proof) -> bool
where
    A: Digest<OutputSize = U64> + Default,
    B: Digest<OutputSize = U32> + Default,
{
    let y = from_bytes(public).expect("valid key");
    let gamma = CompressedRistretto::from_slice(&pi.gamma).decompress();
    if gamma.is_none() {
        return false;
    }
    let gamma = gamma.unwrap();
    let mut hasher = B::default();
    hasher.update(to_bytes(gamma));
    if beta != hasher.finalize_reset().as_slice() {
        return false;
    }
    let s = Scalar::from_canonical_bytes(pi.s);
    if s.is_none() {
        return false;
    }
    let s = s.unwrap();
    let c_scalar = Scalar::from_bytes_mod_order(pi.c);
    let u = y * c_scalar + &RISTRETTO_BASEPOINT_TABLE * &s;
    let h = RistrettoPoint::hash_from_bytes::<A>(alpha);
    let v = gamma * c_scalar + h * s;
    hasher.update([to_bytes(h), to_bytes(y), to_bytes(gamma), to_bytes(u), to_bytes(v)].concat());
    if pi.c != hasher.finalize().as_slice() {
        return false;
    }
    true
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
        let (beta, pi) = prove::<Sha3_512, Sha3_256>(&alpha, &key.scalar);
        assert!(verify::<Sha3_512, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha, &beta, &pi));
    }
    #[test]
    fn test_fake_proof() {
        let key = Key::generate();
        let key_fake = Key::generate();
        let alpha = [0];
        let alpha_fake = [1];
        let (beta, pi) = prove::<Sha3_512, Sha3_256>(&alpha, &key.scalar);
        let (beta_fake_0, pi_fake) = prove::<Sha3_512, Sha3_256>(&alpha, &key_fake.scalar);
        let mut beta_fake_1 = beta.clone();
        beta_fake_1[0] += 0x01;
        assert!(!verify::<Sha3_512, Sha3_256>(key_fake.compressed_ristretto().as_bytes(), &alpha, &beta, &pi));
        assert!(!verify::<Sha3_512, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha_fake, &beta, &pi));
        assert!(!verify::<Sha3_512, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha, &beta_fake_0, &pi));
        assert!(!verify::<Sha3_512, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha, &beta_fake_1, &pi));
        assert!(!verify::<Sha3_512, Sha3_256>(key.compressed_ristretto().as_bytes(), &alpha, &beta, &pi_fake));
    }
    #[test]
    fn test_serialize() {
        let key = Key::generate();
        let alpha = [];
        let (_, pi) = prove::<Sha3_512, Sha3_256>(&alpha, &key.scalar);
        assert_eq!(pi, Proof::from_bytes(&pi.to_bytes()));
    }
    #[test]
    fn test_validate_key() {
        let key = Key::generate();
        assert!(validate_key(key.compressed_ristretto().as_bytes()));
    }
}
