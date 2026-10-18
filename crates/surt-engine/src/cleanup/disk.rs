use std::fs;
use std::io;
use std::path::Path;

use super::{Kept, Sweep};
use crate::project::Error;
use crate::store::WorkingCopyArtifact;
use crate::working_copy::unrecorded_edits;

impl Sweep<'_> {
    pub(super) async fn working_copy(&mut self) -> Result<(), Error> {
        let subject = self.store.subject_of_run(self.root).await?;
        let dir = self.layout.working_copy(&subject);
        for artifact in self.store.working_copy(self.root).await? {
            self.remove_unedited(&dir, &artifact);
        }
        if let Err(error) = prune_empty(&dir) {
            self.leave_path(&dir, error);
        }
        Ok(())
    }

    fn remove_unedited(&mut self, dir: &Path, artifact: &WorkingCopyArtifact) {
        let edited = match unrecorded_edits(dir, &artifact.files) {
            Ok(edited) => edited,
            Err(error) => return self.leave_path(dir, error),
        };
        for file in &artifact.files {
            if edited.contains(&file.rel_path) {
                continue;
            }
            let path = dir.join(&file.rel_path);
            match remove_file(&path) {
                Ok(removed) => self.report.working_copy |= removed,
                Err(error) => self.leave_path(&path, error),
            }
        }
        for rel_path in edited {
            let path = self.relative(&dir.join(rel_path));
            self.report.kept.push(Kept::UnrecordedEdit { path });
        }
    }

    pub(super) async fn executions(&mut self) -> Result<(), Error> {
        for execution in self.store.execution_ids(self.root).await? {
            let path = self.layout.execution(&execution);
            if let Err(error) = remove_dir(&path) {
                self.leave_path(&path, error);
            }
        }
        Ok(())
    }
}

fn remove_file(path: &Path) -> io::Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn remove_dir(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn prune_empty(dir: &Path) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let mut emptied = true;
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            prune_empty(&entry.path())?;
            emptied &= !entry.path().exists();
        } else {
            emptied = false;
        }
    }
    if emptied {
        fs::remove_dir(dir)?;
    }
    Ok(())
}
