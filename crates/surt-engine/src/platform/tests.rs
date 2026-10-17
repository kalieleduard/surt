use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;

use tempfile::TempDir;

use super::lock_exclusive;

fn other_handle(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("the lock file opens")
}

#[test]
fn an_exclusive_lock_creates_its_directories_and_excludes_other_holders() {
    let root = TempDir::new().expect("a temporary directory");
    let path = root.path().join("runtime").join("locks").join("db.lock");

    let held = lock_exclusive(&path).expect("the lock is taken");

    assert!(matches!(
        other_handle(&path).try_lock(),
        Err(TryLockError::WouldBlock)
    ));
    drop(held);
    assert!(other_handle(&path).try_lock().is_ok());
}
