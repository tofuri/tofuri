use crate::constants::AMOUNT_BYTES;
pub fn to_bytes(input: &u128) -> [u8; AMOUNT_BYTES] {
    if input == &0 {
        return [0; AMOUNT_BYTES];
    }
    let bytes = input.to_be_bytes();
    let mut i = 0;
    for byte in bytes {
        if byte != 0 {
            break;
        }
        i += 1;
    }
    let size = 15 - i;
    let mut output = [0; AMOUNT_BYTES];
    for (j, v) in output.iter_mut().enumerate().take(AMOUNT_BYTES) {
        let k = i + j;
        if k == 16 {
            break;
        }
        *v = bytes[k];
    }
    // rounding
    if output[AMOUNT_BYTES - 1] & 0x0f >= 8 {
        output = (u32::from_be_bytes(output) + 8).to_be_bytes();
    }
    output[AMOUNT_BYTES - 1] = (output[AMOUNT_BYTES - 1] & 0xf0) | size as u8;
    output
}
pub fn from_bytes(input: &[u8; AMOUNT_BYTES]) -> u128 {
    let size = input[AMOUNT_BYTES - 1] as usize & 0x0f;
    let mut bytes = [0; 16];
    for (i, v) in input.iter().enumerate().take(AMOUNT_BYTES) {
        let j = 15 - size + i;
        if j == 16 {
            break;
        }
        if i == AMOUNT_BYTES - 1 {
            bytes[j] = v & 0xf0;
            break;
        }
        bytes[j] = *v;
    }
    u128::from_be_bytes(bytes)
}
pub fn floor(input: &u128) -> u128 {
    from_bytes(&to_bytes(input))
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[test]
    fn test_encode() {
        assert_eq!([1, 0, 0, 8], to_bytes(&0x10000000000000000));
    }
    #[test]
    fn test_decode() {
        assert_eq!(0x10000000000000000, from_bytes(&[1, 0, 0, 8]));
    }
    #[test]
    fn test_decode_max() {
        assert_eq!(
            0xfffffff0000000000000000000000000,
            from_bytes(&[0xff, 0xff, 0xff, 0xff])
        );
    }
    #[bench]
    fn bench_encode(b: &mut Bencher) {
        b.iter(|| to_bytes(&0x10000000000000000));
    }
    #[bench]
    fn bench_decode(b: &mut Bencher) {
        let bytes = to_bytes(&0x10000000000000000);
        b.iter(|| from_bytes(&bytes));
    }
}
