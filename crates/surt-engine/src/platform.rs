use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;

pub(crate) struct ExclusiveLock {
    _file: File,
}

pub(crate) fn lock_exclusive(path: &Path) -> io::Result<ExclusiveLock> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.lock()?;
    Ok(ExclusiveLock { _file: file })
}

#[cfg(test)]
mod tests;
