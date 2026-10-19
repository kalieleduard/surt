mod hygiene;
mod schemas;

use std::fs;
use std::io;
use std::path::Path;

use super::super::layout::Layout;
use super::super::{Error, Refusal};
use crate::workflow::{Catalog, InvalidOverride, LocalAgent, LocalFile, Problem};

const AGENT_FILE: &str = "agent.toml";
const INSTRUCTIONS_FILE: &str = "instructions.md";
const AGENTS_DIR: &str = ".surt/agents";
const PLACEHOLDERS: [&str; 2] = [".gitkeep", ".keep"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalFiles {
    pub agent: String,
    pub files: Vec<String>,
}

pub(super) fn read(layout: &Layout, catalog: &Catalog) -> Result<Vec<LocalAgent>, Error> {
    let agents = layout.agents();
    hygiene::check(&agents, AGENTS_DIR, catalog)?;
    catalog
        .agent_ids()
        .map(|id| {
            let dir = agents.join(id);
            let read_named = |name: &str| read_file(&dir.join(name), &shown(id, name));
            Ok(LocalAgent {
                id: id.to_owned(),
                toml: read_named(AGENT_FILE)?,
                instructions: read_named(INSTRUCTIONS_FILE)?,
                schemas: schemas::read(&dir, id)?,
            })
        })
        .collect()
}

fn is_placeholder(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| PLACEHOLDERS.iter().any(|placeholder| name == *placeholder))
        && !path.is_dir()
}

fn shown(id: &str, rel_path: &str) -> String {
    format!("{AGENTS_DIR}/{id}/{rel_path}")
}

pub(super) fn reported(locals: &[LocalAgent]) -> Vec<LocalFiles> {
    locals
        .iter()
        .map(|local| LocalFiles {
            agent: local.id.clone(),
            files: local_files(local),
        })
        .filter(|reported| !reported.files.is_empty())
        .collect()
}

fn local_files(local: &LocalAgent) -> Vec<String> {
    let named = [
        (AGENT_FILE, local.toml.is_some()),
        (INSTRUCTIONS_FILE, local.instructions.is_some()),
    ];
    named
        .into_iter()
        .filter(|(_, present)| *present)
        .map(|(name, _)| name.to_owned())
        .chain(local.schemas.keys().cloned())
        .collect()
}

fn read_file(path: &Path, file: &str) -> Result<Option<LocalFile>, Error> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::io("read", path, error)),
    };
    let text = String::from_utf8(bytes).map_err(|error| {
        Refusal::InvalidAgentOverride(InvalidOverride::new(
            file,
            None,
            Problem::Syntax {
                line: None,
                detail: error.to_string(),
            },
        ))
    })?;
    Ok(Some(LocalFile {
        file: file.to_owned(),
        text,
    }))
}
