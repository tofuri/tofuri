use pea::amount::{from_bytes, to_bytes};
fn main() {
    let a = 0xff000000000000000000000000000000;
    println!("{}", a);
    let b = to_bytes(a);
    println!("{:x?}", b);
    println!("{}", from_bytes(b));
}
