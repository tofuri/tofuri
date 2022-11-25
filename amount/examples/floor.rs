use pea_amount as amount;
fn main() {
    let ints = vec![
        0,
        1,
        10,
        100,
        1000000,
        10000000000000,
        10000000000000000000000000,
        100000000000000000000000000000000000000,
    ];
    for int in ints.iter() {
        println!("{}", amount::floor(int));
    }
    for int in ints.iter() {
        println!("{:x}", amount::floor(int));
    }
    for i in 100000000..1000000000 {
        println!("{} {}", i, amount::floor(&i) <= i);
    }
}
