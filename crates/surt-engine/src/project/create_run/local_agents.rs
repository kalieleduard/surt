use std::fs;
use std::io;
use std::path::Path;

use super::super::layout::Layout;
use super::super::{Error, Refusal};
use crate::workflow::{Catalog, InvalidOverride, LocalAgent, LocalFile, Problem};

const AGENT_FILE: &str = "agent.toml";
const AGENTS_DIR: &str = ".surt/agents";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalFiles {
    pub agent: String,
    pub files: Vec<String>,
}

pub(super) fn read(layout: &Layout, catalog: &Catalog) -> Result<Vec<LocalAgent>, Error> {
    let agents = layout.agents();
    catalog
        .agent_ids()
        .map(|id| {
            Ok(LocalAgent {
                id: id.to_owned(),
                toml: read_file(&agents.join(id).join(AGENT_FILE), &shown(id, AGENT_FILE))?,
            })
        })
        .collect()
}

fn shown(id: &str, rel_path: &str) -> String {
    format!("{AGENTS_DIR}/{id}/{rel_path}")
}

pub(super) fn reported(locals: &[LocalAgent]) -> Vec<LocalFiles> {
    locals
        .iter()
        .filter(|local| local.toml.is_some())
        .map(|local| LocalFiles {
            agent: local.id.clone(),
            files: vec![AGENT_FILE.to_owned()],
        })
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
