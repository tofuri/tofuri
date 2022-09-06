use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;
fn main() {
    let i = 41204000000000000000;
    println!("{}", i);
    let mut v = encode(i);
    println!("{:?}", v);
    println!("{}", decode(&mut v));
}
fn encode(i: u128) -> Vec<u8> {
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
fn decode(v: &mut Vec<u8>) -> u128 {
    let size = v.pop().unwrap();
    v.reverse();
    v.resize(size as usize, 0);
    v.reverse();
    v.resize(16, 0);
    Cursor::new(v).read_u128::<LittleEndian>().unwrap()
}
