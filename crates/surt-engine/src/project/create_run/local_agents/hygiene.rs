use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::is_placeholder;
use crate::project::{Error, Refusal};
use crate::workflow::{Catalog, InvalidOverride, Problem};

const AGENT_FILES: [&str; 2] = ["agent.toml", "instructions.md"];
const SCHEMAS_DIR: &str = "schemas";

struct Entry {
    name: String,
    path: PathBuf,
}

pub(super) fn check(agents: &Path, shown: &str, catalog: &Catalog) -> Result<(), Error> {
    for entry in entries(agents)? {
        let file = format!("{shown}/{}", entry.name);
        if !entry.path.is_dir() {
            return Err(refused(&file, Problem::StrayAgentFile));
        }
        if catalog.agent_ids().all(|id| id != entry.name) {
            let known = catalog.agent_ids().map(str::to_owned).collect();
            return Err(refused(&file, Problem::UnknownAgentDirectory { known }));
        }
        check_agent(&entry.path, &file)?;
    }
    Ok(())
}

fn check_agent(dir: &Path, shown: &str) -> Result<(), Error> {
    for entry in entries(dir)? {
        let file = format!("{shown}/{}", entry.name);
        let is_dir = entry.path.is_dir();
        let allowed = if is_dir {
            entry.name == SCHEMAS_DIR
        } else {
            AGENT_FILES.contains(&entry.name.as_str())
        };
        if allowed {
            continue;
        }
        let stray = if is_dir {
            first_file(&entry.path, &file)?
        } else {
            Some(file)
        };
        if let Some(stray) = stray {
            return Err(refused(&stray, Problem::StrayAgentFile));
        }
    }
    Ok(())
}

fn first_file(dir: &Path, shown: &str) -> Result<Option<String>, Error> {
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

fn entries(dir: &Path) -> Result<Vec<Entry>, Error> {
    let read = match fs::read_dir(dir) {
        Ok(read) => read,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(Error::io("read", dir, error)),
    };
    let mut entries = read
        .map(|entry| {
            entry
                .map(|entry| Entry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: entry.path(),
                })
                .map_err(|error| Error::io("read", dir, error))
        })
        .filter(|entry| {
            entry
                .as_ref()
                .map_or(true, |entry| !is_placeholder(&entry.path))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

fn refused(file: &str, problem: Problem) -> Error {
    Error::Refused(Refusal::InvalidAgentOverride(InvalidOverride::new(
        file, None, problem,
    )))
}
