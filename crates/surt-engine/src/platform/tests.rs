use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

use super::lock_exclusive;

const RELEASE_POLLS: u32 = 200;
const RELEASE_POLL_INTERVAL: Duration = Duration::from_millis(10);

fn other_handle(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("the lock file opens")
}

fn becomes_lockable(path: &Path) -> bool {
    (0..RELEASE_POLLS).any(|_| {
        let taken = other_handle(path).try_lock().is_ok();
        if !taken {
            thread::sleep(RELEASE_POLL_INTERVAL);
        }
        taken
    })
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
    assert!(becomes_lockable(&path));
}
