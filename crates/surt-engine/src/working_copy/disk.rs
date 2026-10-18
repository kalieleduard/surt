use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::Value;

use crate::artifacts::sha256_hex;
use crate::project::Error;
use crate::store::{BaselineFile, Store};

const SEPARATOR: char = '/';

#[derive(Debug, Default)]
pub(super) struct Inspection {
    pub(super) changed: Vec<String>,
    pub(super) extra: Vec<String>,
    pub(super) missing: Vec<String>,
}

impl Inspection {
    pub(super) fn is_edit(&self) -> bool {
        !self.changed.is_empty() || !self.extra.is_empty()
    }

    pub(super) fn edited(&self) -> Vec<String> {
        self.changed.iter().chain(&self.extra).cloned().collect()
    }
}

pub(super) fn inspect(dir: &Path, files: &[BaselineFile]) -> io::Result<Inspection> {
    let mut inspection = Inspection::default();
    for file in files {
        match read_optional(&dir.join(&file.rel_path))? {
            None => inspection.missing.push(file.rel_path.clone()),
            Some(bytes) if sha256_hex(&bytes) != file.sha256 => {
                inspection.changed.push(file.rel_path.clone());
            }
            Some(_) => {}
        }
    }
    let known: BTreeSet<&str> = files.iter().map(|file| file.rel_path.as_str()).collect();
    for directory in owned_directories(files) {
        for rel_path in list(dir, directory)? {
            if !known.contains(rel_path.as_str()) {
                inspection.extra.push(rel_path);
            }
        }
    }
    Ok(inspection)
}

pub(super) async fn index(store: &Store, files: &[BaselineFile]) -> Result<String, Error> {
    if let [only] = files {
        return Ok(only.rel_path.clone());
    }
    for file in files {
        let bytes = store.blob(&file.sha256).await?;
        if let Ok(Value::Array(_)) = serde_json::from_slice(&bytes) {
            return Ok(file.rel_path.clone());
        }
    }
    Ok(files
        .first()
        .map(|file| file.rel_path.clone())
        .unwrap_or_default())
}

pub(super) fn write(target: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, bytes)
}

fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn owned_directories(files: &[BaselineFile]) -> BTreeSet<&str> {
    files
        .iter()
        .filter_map(|file| file.rel_path.split_once(SEPARATOR))
        .map(|(directory, _)| directory)
        .collect()
}

fn list(dir: &Path, directory: &str) -> io::Result<Vec<String>> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_owned()];
    while let Some(relative) = pending.pop() {
        let entries = match fs::read_dir(dir.join(&relative)) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        for entry in entries {
            let entry = entry?;
            let name = format!(
                "{relative}{SEPARATOR}{}",
                entry.file_name().to_string_lossy()
            );
            if entry.file_type()?.is_dir() {
                pending.push(name);
            } else {
                found.push(name);
            }
        }
    }
    found.sort_unstable();
    Ok(found)
}
