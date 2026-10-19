use std::fs;
use std::path::{Path, PathBuf};

use super::is_placeholder;
use crate::workflow::{
    AGENT_FILE, DefinitionFile, FreezeError, INSTRUCTIONS_FILE, InvalidDefinition, Problem,
};

const SCHEMAS_DIR: &str = "schemas";

struct Entry {
    name: String,
    path: PathBuf,
}

pub(super) fn check(dir: &Path, id: &str) -> Result<(), FreezeError> {
    for entry in entries(dir)? {
        let is_dir = entry.path.is_dir();
        let allowed = if is_dir {
            entry.name == SCHEMAS_DIR
        } else {
            [AGENT_FILE, INSTRUCTIONS_FILE].contains(&entry.name.as_str())
        };
        if allowed {
            continue;
        }
        let stray = if is_dir {
            first_file(&entry.path, &entry.name)?
        } else {
            Some(entry.name)
        };
        if let Some(path) = stray {
            return Err(stray_file(id, path));
        }
    }
    Ok(())
}

fn first_file(dir: &Path, shown: &str) -> Result<Option<String>, FreezeError> {
    for entry in entries(dir)? {
        let file = format!("{shown}/{}", entry.name);
        if !entry.path.is_dir() {
            return Ok(Some(file));
        }
        if let Some(found) = first_file(&entry.path, &file)? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

fn entries(dir: &Path) -> Result<Vec<Entry>, FreezeError> {
    let unreadable = |source| FreezeError::Unreadable {
        path: dir.to_path_buf(),
        source,
    };
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let path = entry.path();
        if !is_placeholder(&path) {
            entries.push(Entry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path,
            });
        }
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

fn stray_file(id: &str, path: String) -> FreezeError {
    FreezeError::Invalid(InvalidDefinition::new(
        DefinitionFile::AgentFile {
            agent: id.to_owned(),
            path,
        },
        None,
        Problem::StrayAgentFile,
    ))
}
