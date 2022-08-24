use argon2::{Algorithm, Argon2, Params, ParamsBuilder, Version};
fn params() -> Params {
    let mut builder = ParamsBuilder::new();
    builder.m_cost(1024).unwrap();
    builder.t_cost(1).unwrap();
    builder.p_cost(1).unwrap();
    builder.params().unwrap()
}
pub fn derive(password: &[u8], salt: &[u8]) -> [u8; 32] {
    let ctx = Argon2::new(Algorithm::Argon2id, Version::V0x13, params());
    let mut out = [0u8; 32];
    ctx.hash_password_into(password, salt, &mut out).unwrap();
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[bench]
    fn bench_kdf_derive(b: &mut Bencher) {
        b.iter(|| derive("test".as_bytes(), &[0; 32]));
    }
}
