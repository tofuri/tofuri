use curve25519_dalek::constants::RISTRETTO_BASEPOINT_TABLE;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
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
pub fn prove<D>(alpha: &[u8], secret: &Scalar) -> ([u8; 32], Proof)
where
    D: Digest<OutputSize = U64> + Default,
{
    let h = RistrettoPoint::hash_from_bytes::<D>(alpha);
    let p = &RISTRETTO_BASEPOINT_TABLE * secret;
    let gamma = h * secret;
    let k: Scalar = Scalar::random(&mut OsRng);
    let c = blake3::hash(
        &[
            serialize_point(h),
            serialize_point(p),
            serialize_point(gamma),
            serialize_point(&RISTRETTO_BASEPOINT_TABLE * &k),
            serialize_point(h * k),
        ]
        .concat(),
    )
    .into();
    let c_scalar = Scalar::from_bytes_mod_order(c);
    let s = k - c_scalar * secret;
    let beta = blake3::hash(&serialize_point(gamma)).into();
    (
        beta,
        Proof {
            gamma: gamma.compress().to_bytes(),
            c,
            s: s.to_bytes(),
        },
    )
}
pub fn verify<D>(alpha: &[u8], beta: &[u8; 32], p: RistrettoPoint, pi: &Proof) -> bool
where
    D: Digest<OutputSize = U64> + Default,
{
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
    let u = p * c_scalar + &RISTRETTO_BASEPOINT_TABLE * &s;
    let h = RistrettoPoint::hash_from_bytes::<D>(alpha);
    let v = gamma * c_scalar + h * s;
    beta == blake3::hash(&serialize_point(gamma)).as_bytes()
        && blake3::hash(
            &[
                serialize_point(h),
                serialize_point(p),
                serialize_point(gamma),
                serialize_point(u),
                serialize_point(v),
            ]
            .concat(),
        )
        .as_bytes()
            == &pi.c
}
#[cfg(test)]
mod tests {
    use super::*;
    use pea_key::Key;
    use sha3::Sha3_512;
    #[test]
    fn test_proof() {
        let key = Key::generate();
        let alpha = [];
        let (beta, pi) = prove::<Sha3_512>(&alpha, &key.scalar);
        assert!(verify::<Sha3_512>(&alpha, &beta, key.ristretto_point(), &pi));
    }
    #[test]
    fn test_fake_proof() {
        let key = Key::generate();
        let f_key = Key::generate();
        let alpha = [0];
        let f_alpha = [1];
        let (beta, pi) = prove::<Sha3_512>(&alpha, &key.scalar);
        let (f_beta_0, f_pi) = prove::<Sha3_512>(&alpha, &f_key.scalar);
        let mut f_beta_1 = beta.clone();
        f_beta_1[0] += 0x01;
        assert!(!verify::<Sha3_512>(&f_alpha, &beta, key.ristretto_point(), &pi));
        assert!(!verify::<Sha3_512>(&alpha, &beta, f_key.ristretto_point(), &pi));
        assert!(!verify::<Sha3_512>(&alpha, &f_beta_0, key.ristretto_point(), &pi));
        assert!(!verify::<Sha3_512>(&alpha, &f_beta_1, key.ristretto_point(), &pi));
        assert!(!verify::<Sha3_512>(&alpha, &beta, key.ristretto_point(), &f_pi));
    }
    #[test]
    fn test_serialize() {
        let key = Key::generate();
        let alpha = [];
        let (_, pi) = prove::<Sha3_512>(&alpha, &key.scalar);
        assert_eq!(pi, Proof::from_bytes(&pi.to_bytes()));
    }
}
