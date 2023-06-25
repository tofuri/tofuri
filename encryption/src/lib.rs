use argon2::Algorithm;
use argon2::Argon2;
use argon2::Params;
use argon2::ParamsBuilder;
use argon2::Version;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::aead::KeyInit;
use chacha20poly1305::ChaCha20Poly1305;
use lazy_static::lazy_static;
use rand_core::CryptoRngCore;
pub const M_COST: u32 = 1024;
pub const T_COST: u32 = 1;
pub const P_COST: u32 = 1;
lazy_static! {
    static ref PARAMS: Params = params(M_COST, T_COST, P_COST);
}
pub fn params(m_cost: u32, t_cost: u32, p_cost: u32) -> Params {
    let mut builder = ParamsBuilder::new();
    builder.m_cost(m_cost);
    builder.t_cost(t_cost);
    builder.p_cost(p_cost);
    builder.build().unwrap()
}
pub fn derive(pwd: impl AsRef<[u8]>, salt: impl AsRef<[u8]>) -> [u8; 32] {
    let mut out = [0; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, PARAMS.clone())
        .hash_password_into(pwd.as_ref(), salt.as_ref(), &mut out)
        .unwrap();
    out
}
pub fn encrypt(
    rng: &mut impl CryptoRngCore,
    plaintext: impl AsRef<[u8]>,
    pwd: impl AsRef<[u8]>,
) -> [u8; 92] {
    let salt = {
        let mut dest = [0; 32];
        rng.fill_bytes(&mut dest);
        dest
    };
    let nonce = {
        let mut dest = [0; 12];
        rng.fill_bytes(&mut dest);
        dest.into()
    };
    let key = derive(pwd, &salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&key).unwrap();
    let ciphertext: [u8; 48] = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .unwrap()
        .try_into()
        .unwrap();
    let mut encrypted = [0; 92];
    encrypted[0..32].copy_from_slice(&salt);
    encrypted[32..44].copy_from_slice(&nonce);
    encrypted[44..92].copy_from_slice(&ciphertext);
    encrypted
}
pub fn decrypt(encrypted: &[u8; 92], pwd: impl AsRef<[u8]>) -> Option<[u8; 32]> {
    let salt = &encrypted[0..32];
    let nonce = &encrypted[32..44];
    let ciphertext = &encrypted[44..92];
    let key = derive(pwd, salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&key).unwrap();
    cipher
        .decrypt(nonce.into(), ciphertext)
        .ok()
        .and_then(|vec| vec.try_into().ok())
}
#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;
    #[test]
    fn encryption() {
        let rng = &mut OsRng;
        let pwd = "password";
        let secret = [0; 32];
        let encrypted = encrypt(rng, secret, pwd);
        let decrypted = decrypt(&encrypted, pwd).unwrap();
        assert_eq!(secret, decrypted);
    }
}
