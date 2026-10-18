use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use super::disk;
use crate::artifacts::sha256_hex;
use crate::project::Error;
use crate::store::{PendingFile, Store};

pub(crate) async fn finish_pending_writes(
    store: &Store,
    dir: &Path,
    root: &str,
) -> Result<(), Error> {
    let pending = store.pending_working_copy(root).await?;
    let mut revisions = BTreeMap::new();
    for file in &pending {
        let target = dir.join(&file.rel_path);
        let on_disk =
            disk::read_optional(&target).map_err(|error| Error::io("read", &target, error))?;
        if unwritten(on_disk.as_deref(), file) {
            let bytes = store.blob(&file.pending_sha256).await?;
            disk::write(&target, &bytes).map_err(|error| Error::io("write", &target, error))?;
        }
        revisions.insert(file.pending_revision.as_str(), file.run.as_str());
    }
    for (revision, run) in revisions {
        for rel_path in store.superseded_working_copy_files(revision).await? {
            let target = dir.join(rel_path);
            remove(&target).map_err(|error| Error::io("remove", &target, error))?;
        }
        store.promote_working_copy(run, revision).await?;
    }
    Ok(())
}

fn unwritten(on_disk: Option<&[u8]>, file: &PendingFile) -> bool {
    let Some(bytes) = on_disk else {
        return true;
    };
    let sha256 = sha256_hex(bytes);
    sha256 != file.pending_sha256 && sha256 == file.sha256
}

fn remove(target: &Path) -> io::Result<()> {
    match fs::remove_file(target) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
