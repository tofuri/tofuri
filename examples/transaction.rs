use axiom::{transaction, util};
fn main() {
    let keypair = util::keygen();
    let mut tx = transaction::Transaction::new(keypair.public.to_bytes(), 1, 1);
    tx.sign(&keypair);
    println!("{:?}", tx);
    println!("{:?}", tx.verify());
    println!("{:?}", bincode::serialize(&tx).unwrap());
    println!("{}", bincode::serialize(&tx).unwrap().len());
}
