use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{is_placeholder, read_text};
use crate::workflow::{DefinitionFile, FreezeError};

const SCHEMAS_DIR: &str = "schemas";

struct Walk<'a> {
    agent_dir: &'a Path,
    id: &'a str,
    pending: Vec<String>,
    schemas: BTreeMap<String, String>,
}

pub(super) fn read(agent_dir: &Path, id: &str) -> Result<BTreeMap<String, String>, FreezeError> {
    let mut walk = Walk {
        agent_dir,
        id,
        pending: vec![SCHEMAS_DIR.to_owned()],
        schemas: BTreeMap::new(),
    };
    while let Some(rel_dir) = walk.pending.pop() {
        walk.visit(&rel_dir)?;
    }
    Ok(walk.schemas)
}

impl Walk<'_> {
    fn visit(&mut self, rel_dir: &str) -> Result<(), FreezeError> {
        let dir = self.agent_dir.join(rel_dir);
        for path in entries(&dir)? {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let rel_path = format!("{rel_dir}/{name}");
            if path.is_dir() {
                self.pending.push(rel_path);
            } else if !is_placeholder(&path) {
                self.read_schema(&path, rel_path)?;
            }
        }
        Ok(())
    }

    fn read_schema(&mut self, path: &Path, rel_path: String) -> Result<(), FreezeError> {
        let file = DefinitionFile::AgentFile {
            agent: self.id.to_owned(),
            path: rel_path.clone(),
        };
        if let Some(text) = read_text(path, file)? {
            self.schemas.insert(rel_path, text);
        }
        Ok(())
    }
}

fn entries(dir: &Path) -> Result<Vec<PathBuf>, FreezeError> {
    let unreadable = |source| FreezeError::Unreadable {
        path: dir.to_path_buf(),
        source,
    };
    let read = match fs::read_dir(dir) {
        Ok(read) => read,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(unreadable(error)),
    };
    read.map(|entry| entry.map(|entry| entry.path()).map_err(unreadable))
        .collect()
}
