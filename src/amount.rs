const BYTES: usize = 4;
pub fn to_bytes(input: u128) -> [u8; BYTES] {
    let bytes = input.to_be_bytes();
    let mut i = 0;
    for byte in bytes {
        if byte != 0 {
            break;
        }
        i += 1;
    }
    let size = 16 - i;
    let mut output = [0; BYTES];
    for j in 0..BYTES {
        let k = i + j;
        if k == 16 {
            break;
        }
        output[j] = bytes[k];
    }
    output[BYTES - 1] = (output[BYTES - 1] & 0xf0) | size as u8;
    output
}
pub fn from_bytes(input: [u8; BYTES]) -> u128 {
    let size = input[BYTES - 1] as usize & 0x0f;
    let mut bytes = [0; 16];
    for i in 0..BYTES {
        let j = 16 - size + i;
        if j == 16 {
            break;
        }
        if i == BYTES - 1 {
            bytes[j] = input[i] & 0xf0;
            break;
        }
        bytes[j] = input[i];
    }
    println!("{:x?}", bytes);
    u128::from_be_bytes(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[test]
    fn test_encode() {
        assert_eq!([1, 0, 0, 9], to_bytes(0x10000000000000000));
    }
    #[test]
    fn test_decode() {
        assert_eq!(0x10000000000000000, from_bytes([1, 0, 0, 9]));
    }
    #[test]
    fn test_decode_size_0xf0() {
        assert_eq!(0, from_bytes([0xff, 0xff, 0xff, 0xf0]));
    }
    #[bench]
    fn bench_encode(b: &mut Bencher) {
        b.iter(|| to_bytes(0x10000000000000000));
    }
    #[bench]
    fn bench_decode(b: &mut Bencher) {
        let v = to_bytes(0x10000000000000000);
        b.iter(|| from_bytes(v));
    }
}
