use pea::amount;
fn main() {
    let a = 0xff0000000000000000000000000000;
    println!("{}", a);
    let b = amount::to_bytes(a);
    println!("{:x?}", b);
    println!("{}", amount::from_bytes(b));
}
