use std::fs::{self, File, Metadata, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

mod children;
mod console;
mod executable;
mod process;

pub(crate) use children::{contain, exit_code, interrupt};
pub use console::outputs_utf8 as console_outputs_utf8;
pub use console::stderr_columns;
pub(crate) use executable::find_executable;
pub(crate) use process::{ProcessIdentity, identify, is_alive, stop};

pub(crate) const GIT_SETTINGS: &[(&str, &str)] = if cfg!(windows) {
    &[("core.longpaths", "true")]
} else {
    &[]
};

pub(crate) struct ExclusiveLock {
    _file: File,
}

pub(crate) fn lock_exclusive(path: &Path) -> io::Result<ExclusiveLock> {
    let file = lock_file(path)?;
    file.lock()?;
    Ok(ExclusiveLock { _file: file })
}

pub(crate) fn try_lock_exclusive(path: &Path) -> io::Result<Option<ExclusiveLock>> {
    let file = lock_file(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(ExclusiveLock { _file: file })),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

fn lock_file(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

pub(crate) fn is_link(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink() || is_reparse_point(metadata)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests;
