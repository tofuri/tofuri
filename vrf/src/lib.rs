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
fn serialize_point(ristretto_point: RistrettoPoint) -> [u8; 32] {
    ristretto_point.compress().to_bytes()
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
            serialize_point(h),
            serialize_point(p),
            serialize_point(gamma),
            serialize_point(&RISTRETTO_BASEPOINT_TABLE * &k),
            serialize_point(h * k),
        ]
        .concat(),
    );
    let c = hasher.finalize().into();
    let c_scalar = Scalar::from_bytes_mod_order(c);
    let s = k - c_scalar * secret;
    let mut hasher = B::default();
    hasher.update(serialize_point(gamma));
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
pub fn verify<A, B>(alpha: &[u8], beta: &[u8; 32], compressed_ristretto: CompressedRistretto, pi: &Proof) -> bool
where
    A: Digest<OutputSize = U64> + Default,
    B: Digest<OutputSize = U32> + Default,
{
    let y = compressed_ristretto.decompress();
    if y.is_none() {
        return false;
    }
    let y = y.unwrap();
    let gamma = CompressedRistretto::from_slice(&pi.gamma).decompress();
    if gamma.is_none() {
        return false;
    }
    let gamma = gamma.unwrap();
    let s = Scalar::from_canonical_bytes(pi.s);
    if s.is_none() {
        return false;
    }
    let s = s.unwrap();
    let c_scalar = Scalar::from_bytes_mod_order(pi.c);
    let u = y * c_scalar + &RISTRETTO_BASEPOINT_TABLE * &s;
    let h = RistrettoPoint::hash_from_bytes::<A>(alpha);
    let v = gamma * c_scalar + h * s;
    let mut hasher = B::default();
    hasher.update(serialize_point(gamma));
    if beta != hasher.finalize_reset().as_slice() {
        return false;
    }
    hasher.update(
        [
            serialize_point(h),
            serialize_point(y),
            serialize_point(gamma),
            serialize_point(u),
            serialize_point(v),
        ]
        .concat(),
    );
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
        assert!(verify::<Sha3_512, Sha3_256>(&alpha, &beta, key.compressed_ristretto(), &pi));
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
        assert!(!verify::<Sha3_512, Sha3_256>(&alpha_fake, &beta, key.compressed_ristretto(), &pi));
        assert!(!verify::<Sha3_512, Sha3_256>(&alpha, &beta, key_fake.compressed_ristretto(), &pi));
        assert!(!verify::<Sha3_512, Sha3_256>(&alpha, &beta_fake_0, key.compressed_ristretto(), &pi));
        assert!(!verify::<Sha3_512, Sha3_256>(&alpha, &beta_fake_1, key.compressed_ristretto(), &pi));
        assert!(!verify::<Sha3_512, Sha3_256>(&alpha, &beta, key.compressed_ristretto(), &pi_fake));
    }
    #[test]
    fn test_serialize() {
        let key = Key::generate();
        let alpha = [];
        let (_, pi) = prove::<Sha3_512, Sha3_256>(&alpha, &key.scalar);
        assert_eq!(pi, Proof::from_bytes(&pi.to_bytes()));
    }
}
