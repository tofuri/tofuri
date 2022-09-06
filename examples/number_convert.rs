use pea::number_convert::{decode, encode};
fn main() {
    let i = 41204000000000000000;
    println!("{}", i);
    let mut v = encode(i);
    println!("{:?}", v);
    println!("{}", decode(&mut v));
}
