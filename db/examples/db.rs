use rocksdb::IteratorMode;
use tofuri_db as db;
fn main() {
    let db = db::open("./tofuri-db");
    for name in [
        "blocks",
        "transactions",
        "stakes",
        "peers",
        "input addresses",
        "input public keys",
        "betas",
    ] {
        println!(
            "{}: {}",
            name,
            db.iterator_cf(db.cf_handle(name).unwrap(), IteratorMode::Start)
                .count()
        );
    }
}
