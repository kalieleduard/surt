use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use super::super::super::Error;
use super::{is_placeholder, read_file, shown};
use crate::workflow::LocalFile;

const SCHEMAS_DIR: &str = "schemas";

pub(super) fn read(agent_dir: &Path, id: &str) -> Result<BTreeMap<String, LocalFile>, Error> {
    let mut schemas = BTreeMap::new();
    let mut pending = vec![SCHEMAS_DIR.to_owned()];
    while let Some(rel_dir) = pending.pop() {
        let dir = agent_dir.join(&rel_dir);
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(Error::io("read", &dir, error)),
        };
        for entry in entries {
            let entry = entry.map_err(|error| Error::io("read", &dir, error))?;
            let rel_path = format!("{rel_dir}/{}", entry.file_name().to_string_lossy());
            if entry.path().is_dir() {
                pending.push(rel_path);
            } else if is_placeholder(&entry.path()) {
                continue;
            } else if let Some(file) = read_file(&entry.path(), &shown(id, &rel_path))? {
                schemas.insert(rel_path, file);
            }
        }
    }
    Ok(schemas)
}
