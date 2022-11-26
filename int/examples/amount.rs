use pea_int as int;
fn main() {
    let ints = vec![
        0,
        1,
        10,
        100,
        100000000,
        0xff0000000000,
        0xff0000000000000000000000000000,
        0xff000000000000000000000000000000,
    ];
    for int in ints {
        println!("{}", int);
        let bytes = int::to_bytes(int);
        println!("{:x?}", bytes);
        println!("{}", int::from_bytes(&bytes));
        println!()
    }
}
