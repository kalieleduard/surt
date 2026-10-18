use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Value, json};

use super::disk;
use super::lines;
use crate::project::Error;
use crate::store::{BaselineFile, Store};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub change: Change,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Changed { added: usize, removed: usize },
    New,
    Missing,
}

pub(crate) async fn impact(
    store: &Store,
    dir: &Path,
    files: &[BaselineFile],
) -> Result<Vec<FileChange>, Error> {
    let inspection = disk::inspect(dir, files).map_err(|error| Error::io("read", dir, error))?;
    let mut changes = Vec::new();
    for file in files {
        if inspection.changed.contains(&file.rel_path) {
            changes.push(changed(store, dir, file).await?);
        } else if inspection.missing.contains(&file.rel_path) {
            changes.push(found(&file.rel_path, Change::Missing));
        }
    }
    let extra = inspection.extra.iter().map(|path| found(path, Change::New));
    changes.extend(extra);
    changes.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(changes)
}

pub(crate) async fn rewrite(
    store: &Store,
    dir: &Path,
    files: &[BaselineFile],
    changes: &[FileChange],
) -> Result<(), Error> {
    for file in files {
        if !changes.iter().any(|change| change.path == file.rel_path) {
            continue;
        }
        let bytes = store.blob(&file.sha256).await?;
        let target = dir.join(&file.rel_path);
        disk::write(&target, &bytes).map_err(|error| Error::io("write", &target, error))?;
    }
    for change in changes.iter().filter(|change| change.change == Change::New) {
        let target = dir.join(&change.path);
        remove(&target).map_err(|error| Error::io("remove", &target, error))?;
    }
    Ok(())
}

pub(crate) fn details(artifact: &str, revision: u32, changes: &[FileChange]) -> Value {
    let paths = |wanted: fn(&Change) -> bool| -> Vec<&str> {
        changes
            .iter()
            .filter(|change| wanted(&change.change))
            .map(|change| change.path.as_str())
            .collect()
    };
    json!({
        "artifact": artifact,
        "revision": revision,
        "changed": paths(|change| matches!(change, Change::Changed { .. })),
        "new": paths(|change| *change == Change::New),
        "missing": paths(|change| *change == Change::Missing),
    })
}

async fn changed(store: &Store, dir: &Path, file: &BaselineFile) -> Result<FileChange, Error> {
    let recorded = store.blob(&file.sha256).await?;
    let target = dir.join(&file.rel_path);
    let on_disk = fs::read(&target).map_err(|error| Error::io("read", &target, error))?;
    let counts = lines::counts(&recorded, &on_disk);
    Ok(found(
        &file.rel_path,
        Change::Changed {
            added: counts.added,
            removed: counts.removed,
        },
    ))
}

fn found(path: &str, change: Change) -> FileChange {
    FileChange {
        path: path.to_owned(),
        change,
    }
}

fn remove(target: &Path) -> io::Result<()> {
    match fs::remove_file(target) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
