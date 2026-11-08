use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use surt_engine::project::Project;
use tauri::State;

use crate::workspace::{Failure, Workspace};

const DOT_SURT: &str = ".surt";
const AGENTS: &str = "agents";
const WORKFLOWS: &str = "workflows";
const WORKFLOW_EXTENSION: &str = ".toml";
const AGENT_FILE: &str = "agent.toml";
const INSTRUCTIONS_FILE: &str = "instructions.md";
const REMOVABLE_DIRS: [&str; 2] = ["references", "schemas"];
const UNSAFE_CHARS: [char; 2] = ['\\', ':'];

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    agents: Vec<AgentEntry>,
    workflows: Vec<WorkflowEntry>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    id: String,
    files: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEntry {
    name: String,
    file: String,
    problem: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Place {
    Workflow,
    AgentCore,
    AgentExtra,
}

#[tauri::command]
pub async fn definitions(workspace: State<'_, Workspace>) -> Result<Catalog, Failure> {
    let project = workspace.project().await?;
    catalog(&project)
}

#[tauri::command]
pub async fn read_definition(
    path: String,
    workspace: State<'_, Workspace>,
) -> Result<String, Failure> {
    let project = workspace.project().await?;
    let (file, _) = resolve(&surt_dir(&project), &path)?;
    fs::read_to_string(&file).map_err(|error| Failure::Io(path, error.to_string()))
}

#[tauri::command]
pub async fn save_definition(
    path: String,
    content: String,
    workspace: State<'_, Workspace>,
) -> Result<Catalog, Failure> {
    let project = workspace.project().await?;
    let (file, _) = resolve(&surt_dir(&project), &path)?;
    write(&file, &content).map_err(|error| Failure::Io(path, error.to_string()))?;
    catalog(&project)
}

#[tauri::command]
pub async fn delete_definition(
    path: String,
    workspace: State<'_, Workspace>,
) -> Result<Catalog, Failure> {
    let project = workspace.project().await?;
    let (file, place) = resolve(&surt_dir(&project), &path)?;
    if place != Place::AgentExtra {
        return Err(Failure::NotRemovable(path));
    }
    fs::remove_file(&file).map_err(|error| Failure::Io(path, error.to_string()))?;
    catalog(&project)
}

fn surt_dir(project: &Project) -> PathBuf {
    project.root().join(DOT_SURT)
}

fn catalog(project: &Project) -> Result<Catalog, Failure> {
    let surt = surt_dir(project);
    let workflows = project
        .workflow_definitions()?
        .into_iter()
        .map(|summary| WorkflowEntry {
            file: format!("{WORKFLOWS}/{}{WORKFLOW_EXTENSION}", summary.name),
            problem: summary.run_inputs.err().map(|error| error.to_string()),
            name: summary.name,
        })
        .collect();
    let agents = agent_entries(&surt.join(AGENTS))
        .map_err(|error| Failure::Io(AGENTS.to_owned(), error.to_string()))?;
    Ok(Catalog { agents, workflows })
}

fn agent_entries(dir: &Path) -> io::Result<Vec<AgentEntry>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut agents = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        collect_files(&entry.path(), "", &mut files)?;
        files.sort_by_key(|file| (file_rank(file), file.clone()));
        agents.push(AgentEntry {
            id: entry.file_name().to_string_lossy().into_owned(),
            files,
        });
    }
    agents.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(agents)
}

fn collect_files(dir: &Path, prefix: &str, files: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}{name}");
        if entry.file_type()?.is_dir() {
            collect_files(&entry.path(), &format!("{relative}/"), files)?;
        } else {
            files.push(relative);
        }
    }
    Ok(())
}

fn file_rank(file: &str) -> u8 {
    match file {
        INSTRUCTIONS_FILE => 0,
        AGENT_FILE => 1,
        _ => 2,
    }
}

pub fn resolve(surt: &Path, relative: &str) -> Result<(PathBuf, Place), Failure> {
    let outside = || Failure::OutsideDefinitions(relative.to_owned());
    let parts: Vec<&str> = relative.split('/').collect();
    let safe = parts.iter().all(|part| {
        !part.is_empty() && *part != "." && *part != ".." && !part.contains(UNSAFE_CHARS)
    });
    if !safe {
        return Err(outside());
    }
    let place = match parts.as_slice() {
        [WORKFLOWS, file]
            if file.ends_with(WORKFLOW_EXTENSION) && file.len() > WORKFLOW_EXTENSION.len() =>
        {
            Place::Workflow
        }
        [AGENTS, _, AGENT_FILE | INSTRUCTIONS_FILE] => Place::AgentCore,
        [AGENTS, _, dir, _, ..] if REMOVABLE_DIRS.contains(dir) => Place::AgentExtra,
        _ => return Err(outside()),
    };
    let path = parts
        .iter()
        .fold(surt.to_path_buf(), |path, part| path.join(part));
    Ok((path, place))
}

fn write(file: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(file, content)
}

#[cfg(test)]
mod tests;
