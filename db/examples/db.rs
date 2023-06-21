use rocksdb::IteratorMode;
use tofuri_db as db;
fn main() {
    let db = db::open_cf_descriptors("./tofuri-db");
    for name in ["blocks", "transactions", "stakes", "peers"] {
        println!(
            "{}: {}",
            name,
            db.iterator_cf(db.cf_handle(name).unwrap(), IteratorMode::Start)
                .count()
        );
    }
}
