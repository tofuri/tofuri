use axiom::{util, wallet::address};
fn main() {
    let keypair = util::keygen();
    println!("{}", address::encode(&keypair.public.as_bytes()));
}
