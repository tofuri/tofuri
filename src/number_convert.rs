use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;
pub fn encode(i: u128) -> Vec<u8> {
    // convert u128 to vec
    let mut v = vec![];
    v.write_u128::<LittleEndian>(i).unwrap();
    // remove subsequent zeroes
    let mut i = 16;
    for b in v.iter().rev() {
        if b != &0 {
            break;
        }
        i -= 1;
    }
    v.drain(i..);
    // size of the number
    v.push(v.len() as u8);
    // remove preceding zeroes
    let mut i: u8 = 0;
    for b in v.iter() {
        if b != &0 {
            break;
        }
        i += 1;
    }
    v.drain(0..i as usize);
    v
}
pub fn decode(v: &mut Vec<u8>) -> u128 {
    let size = v.pop().unwrap();
    v.reverse();
    v.resize(size as usize, 0);
    v.reverse();
    v.resize(16, 0);
    Cursor::new(v).read_u128::<LittleEndian>().unwrap()
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[test]
    fn test_encode() {
        assert_eq!(vec![0xff, 9], encode(0xff0000000000000000));
    }
    #[test]
    fn test_decode() {
        assert_eq!(0xff0000000000000000, decode(&mut vec![0xff, 9]));
    }
    #[bench]
    fn bench_encode(b: &mut Bencher) {
        b.iter(|| encode(0xff0000000000000000));
    }
    #[bench]
    fn bench_decode(b: &mut Bencher) {
        let v = encode(0xff0000000000000000);
        b.iter(|| decode(&mut v.clone()));
    }
}
