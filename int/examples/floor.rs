use pea_int as int;
fn main() {
    let ints = vec![
        0,
        1,
        10,
        100,
        1000000,
        10000000000000,
        1000000000000000000,
        10000000000000000000000000,
        100000000000000000000000000000000000000,
    ];
    for int in ints.clone() {
        println!("{}", int::floor(int));
    }
    for int in ints {
        println!("{:x}", int::floor(int));
    }
}
