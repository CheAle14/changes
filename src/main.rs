use std::path::PathBuf;

use changes::get_hash;

fn main() {
    let mut args = std::env::args();
    let _ = args.next().unwrap();
    let path = args.next().unwrap();
    let path = PathBuf::from(path);
    let hash = get_hash(path).unwrap();
    println!("{}", hash.to_hex());
}
