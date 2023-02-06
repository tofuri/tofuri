use pea_db as db;
use rocksdb::IteratorMode;
fn main() {
    let db = db::open("./peacash-db");
    for name in ["blocks", "transactions", "stakes", "peers", "input addresses", "input public keys", "betas"] {
        println!("{}: {}", name, db.iterator_cf(db.cf_handle(name).unwrap(), IteratorMode::Start).count());
    }
}
