use std::{
    cell::Cell,
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use blake3::Hash;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use gitignore::IgnoreChecker;

mod gitignore;

pub fn get_store_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join(".changes.hash")
    } else if let Some(filename) = path.file_name() {
        let mut owned = OsString::new();
        owned.push(".");
        owned.push(filename);
        owned.push(".changes.hash");
        path.with_file_name(owned)
    } else {
        path.with_file_name(".ERR.changes.hash")
    }
}

pub fn has_changes(check_path: impl AsRef<Path>) -> std::io::Result<bool> {
    let path = check_path.as_ref();
    let store = get_store_path(path);
    let hash = get_hash(path)?;

    if store.exists() {
        let stored = std::fs::read_to_string(&store)?;
        let stored = blake3::Hash::from_hex(stored).unwrap();

        if hash == stored {
            Ok(false)
        } else {
            std::fs::write(store, hash.to_hex().as_bytes())?;
            Ok(true)
        }
    } else {
        std::fs::write(store, hash.to_hex().as_bytes())?;
        Ok(true)
    }
}

pub fn get_hash<P>(path: P) -> std::io::Result<Hash>
where
    P: AsRef<Path>,
{
    _get_hash(path.as_ref())
}

struct HashRequest {
    pub path: PathBuf,
    pub callback: Arc<RwLock<Option<Hash>>>,
}

fn get_ignore_for_root(path: &Path) -> Arc<IgnoreChecker> {
    if path.is_dir() {
        let path = path.join(".gitignore");
        if path.exists() {
            return IgnoreChecker::root(path);
        }
    }
    IgnoreChecker::empty()
}

fn _get_hash(path: &Path) -> std::io::Result<Hash> {
    let (tx, rx) = crossbeam_channel::unbounded();

    let ignore = get_ignore_for_root(path);

    for _ in 0..6 {
        let tx_c = tx.clone();
        let rx_c = rx.clone();
        let ig_c = ignore.clone();
        std::thread::spawn(move || {
            let tx = tx_c;
            let rx = rx_c;
            let ignore = ig_c;

            let buffer = vec![0u8; 1024 * 16];
            let mut buffer = buffer.into_boxed_slice();

            while let Ok(next) = rx.recv() {
                progress_request(&next, &tx, &rx, &ignore, &mut buffer).unwrap();
            }
        });
    }

    let buffer = vec![0u8; 1024 * 16];
    let mut buffer = buffer.into_boxed_slice();
    get_hash_dir(path, &tx, &rx, &ignore, &mut buffer)
}

fn progress_request(
    next: &HashRequest,
    tx: &Sender<HashRequest>,
    rx: &Receiver<HashRequest>,
    ignore: &Arc<IgnoreChecker>,
    buffer: &mut [u8],
) -> std::io::Result<()> {
    if next.path.is_file() {
        let hash = get_hash_file(&next.path, buffer)?;
        let mut wr = next.callback.write().unwrap();
        *wr = Some(hash);
        Ok(())
    } else {
        let hash = get_hash_dir(&next.path, tx, rx, ignore, buffer)?;
        let mut wr = next.callback.write().unwrap();
        *wr = Some(hash);
        Ok(())
    }
}

fn get_hash_file(path: &Path, buffer: &mut [u8]) -> std::io::Result<Hash> {
    let mut file = std::fs::File::open(path)?;

    let mut hasher = blake3::Hasher::new();
    if let Some(filename) = path.file_name() {
        hasher.update(filename.as_encoded_bytes());
    }

    loop {
        let result = file.read(buffer)?;
        if result == 0 {
            break;
        }
        let read = &buffer[..result];
        hasher.update(read);
    }

    Ok(hasher.finalize())
}
//
fn get_hash_dir(
    path: &Path,
    tx: &Sender<HashRequest>,
    rx: &Receiver<HashRequest>,
    ignore: &Arc<IgnoreChecker>,
    buffer: &mut [u8],
) -> std::io::Result<Hash> {
    let ignore = {
        let ignore_path = path.join(".gitignore");
        if ignore_path.exists() {
            ignore.child(ignore_path)
        } else {
            Arc::clone(ignore)
        }
    };

    let mut callbacks = Vec::new();
    for entry in path.read_dir()? {
        let entry = entry?;
        let entry_path = entry.path();
        if ignore.should_ignore(&entry_path) || entry.file_name().to_string_lossy().starts_with('.')
        {
            continue;
        }
        let cb = Arc::new(RwLock::new(Option::<Hash>::None));
        let req = HashRequest {
            path: entry_path,
            callback: cb.clone(),
        };
        tx.send(req).unwrap();
        callbacks.push(cb);
    }

    loop {
        match rx.try_recv() {
            Ok(next) => {
                progress_request(&next, tx, rx, &ignore, buffer)?;
            }
            Err(_) if is_done(&callbacks) => {
                break;
            }
            Err(TryRecvError::Empty) => continue,
            Err(TryRecvError::Disconnected) => {
                // we still have a ref to a sender
                unreachable!()
            }
        }
    }

    let mut hasher = blake3::Hasher::new();
    for cb in callbacks {
        let r = cb.read().unwrap();
        let hash = r.unwrap();
        hasher.update(hash.as_bytes());
    }

    Ok(hasher.finalize())
}

fn is_done(cbs: &[Arc<RwLock<Option<Hash>>>]) -> bool {
    cbs.iter().all(|o| {
        let r = o.read().unwrap();
        r.is_some()
    })
}
