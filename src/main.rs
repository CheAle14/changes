use std::path::PathBuf;

use changes::{get_hash, get_store_path};

fn main() {
    let mut args = std::env::args();
    let _ = args.next().unwrap();
    let path = args.next().unwrap();
    let path = PathBuf::from(path);
    let hash = get_hash(&path).unwrap();
    println!("Current  {}", hash.to_hex());

    let store = get_store_path(&path);
    if store.exists() {
        let stored = std::fs::read_to_string(&store).unwrap();
        let stored = blake3::Hash::from_hex(stored).unwrap();
        println!("Previous {}", stored.to_hex());

        println!("Eq? {}", hash == stored);
    } else {
        println!("No previous");
    }
}
