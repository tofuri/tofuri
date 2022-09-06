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
    let mut v = vec![];
    v.write_u128::<LittleEndian>(i).unwrap();
    let mut i = 16;
    for b in v.iter().rev() {
        if b != &0 {
            break;
        }
        i -= 1;
    }
    v.drain(i..);
    let len = v.len();
    let mut i: u8 = 0;
    for b in v.iter() {
        if b != &0 {
            break;
        }
        i += 1;
    }
    v.drain(0..i as usize);
    v.push(len as u8);
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
