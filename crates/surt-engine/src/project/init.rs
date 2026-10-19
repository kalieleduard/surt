use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use super::config::TEMPLATE as CONFIG_TEMPLATE;
use super::discovery::{self, SurtHome};
use super::error::{Error, Refusal};
use super::layout::Layout;
use crate::git::{self, GitVersion, MIN_VERSION};
use crate::store::Store;

const GITIGNORE: &str = include_str!("../../assets/gitignore");

const TEMPLATE_SEPARATOR: char = '/';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitFile {
    Gitignore,
    Config,
    Database,
    Template(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    Created,
    Kept,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Initialized {
    pub dot_surt: PathBuf,
    pub from_subdirectory: bool,
    pub files: Vec<(InitFile, FileState)>,
    pub git: GitVersion,
    pub identity: String,
}

impl Initialized {
    pub fn was_already_initialized(&self) -> bool {
        self.files
            .iter()
            .any(|(file, state)| *file == InitFile::Database && *state == FileState::Kept)
    }
}

pub(crate) async fn run(
    cwd: &Path,
    home: &SurtHome,
    template: &[(&str, &[u8])],
) -> Result<Initialized, Error> {
    let cwd = discovery::outside_managed_worktrees(cwd, home).await?;
    let git = supported_git().await?;
    let Some(root) = discovery::main_toplevel(&cwd).await? else {
        return Err(Refusal::NotAGitRepository { path: cwd }.into());
    };
    let identity = git::committer_email(&root)
        .await?
        .ok_or(Refusal::NoGitIdentity)?;
    let layout = Layout::new(root);
    let mut files = create_files(&layout).await?;
    files.extend(write_template(&layout, template)?);
    Ok(Initialized {
        dot_surt: layout.dot_surt(),
        from_subdirectory: cwd != layout.root(),
        files,
        git,
        identity,
    })
}

pub(crate) async fn supported_git() -> Result<GitVersion, Error> {
    let git = git::version().await?;
    if git < MIN_VERSION {
        return Err(Refusal::GitTooOld {
            found: git,
            min: MIN_VERSION,
        }
        .into());
    }
    Ok(git)
}

async fn create_files(layout: &Layout) -> Result<Vec<(InitFile, FileState)>, Error> {
    let dot_surt = layout.dot_surt();
    fs::create_dir_all(&dot_surt).map_err(|error| Error::io("create", &dot_surt, error))?;
    let gitignore = write_if_missing(&layout.gitignore(), GITIGNORE.as_bytes())?;
    let config = write_if_missing(&layout.config(), CONFIG_TEMPLATE.as_bytes())?;
    let database = open_database(layout).await?;
    Ok(vec![
        (InitFile::Gitignore, gitignore),
        (InitFile::Config, config),
        (InitFile::Database, database),
    ])
}

fn write_template(
    layout: &Layout,
    template: &[(&str, &[u8])],
) -> Result<Vec<(InitFile, FileState)>, Error> {
    let dot_surt = layout.dot_surt();
    template
        .iter()
        .map(|(rel_path, bytes)| {
            let path = template_path(&dot_surt, rel_path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| Error::io("create", parent, error))?;
            }
            let state = write_if_missing(&path, bytes)?;
            Ok((InitFile::Template((*rel_path).to_owned()), state))
        })
        .collect()
}

fn template_path(dot_surt: &Path, rel_path: &str) -> Result<PathBuf, Error> {
    let relative: PathBuf = rel_path.split(TEMPLATE_SEPARATOR).collect();
    let inside = !rel_path.is_empty()
        && relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if !inside {
        let error = io::Error::new(io::ErrorKind::InvalidInput, "outside .surt/");
        return Err(Error::io("create", &relative, error));
    }
    Ok(dot_surt.join(relative))
}

fn write_if_missing(path: &Path, contents: &[u8]) -> Result<FileState, Error> {
    let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(FileState::Kept),
        Err(error) => return Err(Error::io("create", path, error)),
    };
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|error| Error::io("write", path, error))?;
    Ok(FileState::Created)
}

async fn open_database(layout: &Layout) -> Result<FileState, Error> {
    let database = layout.database();
    let state = if database.is_file() {
        FileState::Kept
    } else {
        FileState::Created
    };
    Store::open(&database, &layout.database_lock(), &layout.project_name())
        .await?
        .close()
        .await;
    Ok(state)
}
