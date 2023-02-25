use tofuri_int as int;
fn main() {
    for i in 0..39 {
        let x = 10_u128.pow(i);
        let y = int::floor(x);
        let z = y as f64 / x as f64;
        println!("{y} {z}");
    }
}
