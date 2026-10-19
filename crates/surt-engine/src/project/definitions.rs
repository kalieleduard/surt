mod hygiene;
mod schemas;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::error::{Error, Refusal};
use super::layout::Layout;
use crate::workflow::{
    AGENT_FILE, AgentFiles, DefinitionFile, DefinitionSource, FreezeError, INSTRUCTIONS_FILE,
    InvalidDefinition, Problem, validate,
};

const WORKFLOW_EXTENSION: &str = "toml";
const PLACEHOLDERS: [&str; 2] = [".gitkeep", ".keep"];
const UNSAFE_NAME_CHARS: [char; 3] = ['/', '\\', ':'];

pub(crate) struct ProjectDefinitions {
    workflows: PathBuf,
    agents: PathBuf,
}

pub fn check_workflow(root: &Path, workflow: &str) -> Result<(), Error> {
    let definitions = ProjectDefinitions::new(&Layout::new(root.to_path_buf()));
    definitions.require(workflow)?;
    validate(&definitions, workflow)?;
    Ok(())
}

impl ProjectDefinitions {
    pub(crate) fn new(layout: &Layout) -> ProjectDefinitions {
        ProjectDefinitions {
            workflows: layout.workflows(),
            agents: layout.agents(),
        }
    }

    pub(crate) fn require(&self, workflow: &str) -> Result<(), Error> {
        if is_plain_name(workflow) && self.workflow_path(workflow).is_file() {
            return Ok(());
        }
        Err(Refusal::UnknownWorkflow {
            name: workflow.to_owned(),
            known: self.workflow_names()?,
        }
        .into())
    }

    fn workflow_names(&self) -> Result<Vec<String>, Error> {
        let entries = match fs::read_dir(&self.workflows) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(Error::io("read", &self.workflows, error)),
        };
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| Error::io("read", &self.workflows, error))?;
            names.extend(workflow_stem(&entry.path()));
        }
        names.sort();
        Ok(names)
    }

    fn workflow_path(&self, name: &str) -> PathBuf {
        self.workflows.join(format!("{name}.{WORKFLOW_EXTENSION}"))
    }
}

impl DefinitionSource for ProjectDefinitions {
    fn workflow(&self, name: &str) -> Result<Option<String>, FreezeError> {
        if !is_plain_name(name) {
            return Ok(None);
        }
        read_text(
            &self.workflow_path(name),
            DefinitionFile::Workflow(name.to_owned()),
        )
    }

    fn agent(&self, id: &str) -> Result<Option<AgentFiles>, FreezeError> {
        let dir = self.agents.join(id);
        if !is_plain_name(id) || !dir.is_dir() {
            return Ok(None);
        }
        hygiene::check(&dir, id)?;
        let agent_file = |name: &str| DefinitionFile::AgentFile {
            agent: id.to_owned(),
            path: name.to_owned(),
        };
        Ok(Some(AgentFiles {
            toml: required(&dir.join(AGENT_FILE), DefinitionFile::Agent(id.to_owned()))?,
            instructions: required(&dir.join(INSTRUCTIONS_FILE), agent_file(INSTRUCTIONS_FILE))?,
            schemas: schemas::read(&dir, id)?,
        }))
    }
}

fn workflow_stem(path: &Path) -> Option<String> {
    let is_workflow = path.is_file() && path.extension()? == WORKFLOW_EXTENSION;
    is_workflow.then(|| {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
    })?
}

fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(UNSAFE_NAME_CHARS)
}

fn is_placeholder(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| PLACEHOLDERS.iter().any(|placeholder| name == *placeholder))
        && !path.is_dir()
}

fn required(path: &Path, file: DefinitionFile) -> Result<String, FreezeError> {
    read_text(path, file.clone())?.ok_or_else(|| {
        FreezeError::Invalid(InvalidDefinition::new(file, None, Problem::MissingFile))
    })
}

fn read_text(path: &Path, file: DefinitionFile) -> Result<Option<String>, FreezeError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(FreezeError::Unreadable {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    String::from_utf8(bytes).map(Some).map_err(|error| {
        FreezeError::Invalid(InvalidDefinition::new(
            file,
            None,
            Problem::Syntax {
                line: None,
                detail: error.to_string(),
            },
        ))
    })
}
