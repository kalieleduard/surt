use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::config::TEMPLATE as CONFIG_TEMPLATE;
use super::discovery::{self, SurtHome};
use super::error::{Error, Refusal};
use super::layout::Layout;
use crate::git::{self, GitVersion, MIN_VERSION};
use crate::store::Store;

const GITIGNORE: &str = include_str!("../../assets/gitignore");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitFile {
    Gitignore,
    Config,
    Database,
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
    pub files: [(InitFile, FileState); 3],
    pub git: GitVersion,
    pub identity: String,
}

impl Initialized {
    pub fn was_already_initialized(&self) -> bool {
        self.files
            .iter()
            .all(|(_, state)| *state == FileState::Kept)
    }
}

pub(crate) async fn run(cwd: &Path, home: &SurtHome) -> Result<Initialized, Error> {
    let cwd = discovery::outside_managed_worktrees(cwd, home).await?;
    let git = git::version().await?;
    if git < MIN_VERSION {
        return Err(Refusal::GitTooOld {
            found: git,
            min: MIN_VERSION,
        }
        .into());
    }
    let Some(root) = discovery::main_toplevel(&cwd).await? else {
        return Err(Refusal::NotAGitRepository { path: cwd }.into());
    };
    let identity = git::committer_email(&root)
        .await?
        .ok_or(Refusal::NoGitIdentity)?;
    let layout = Layout::new(root);
    let files = create_files(&layout).await?;
    Ok(Initialized {
        dot_surt: layout.dot_surt(),
        from_subdirectory: cwd != layout.root(),
        files,
        git,
        identity,
    })
}

async fn create_files(layout: &Layout) -> Result<[(InitFile, FileState); 3], Error> {
    let dot_surt = layout.dot_surt();
    fs::create_dir_all(&dot_surt).map_err(|error| Error::io("create", &dot_surt, error))?;
    let gitignore = write_if_missing(&layout.gitignore(), GITIGNORE)?;
    let config = write_if_missing(&layout.config(), CONFIG_TEMPLATE)?;
    let database = open_database(layout).await?;
    Ok([
        (InitFile::Gitignore, gitignore),
        (InitFile::Config, config),
        (InitFile::Database, database),
    ])
}

fn write_if_missing(path: &Path, contents: &str) -> Result<FileState, Error> {
    let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(FileState::Kept),
        Err(error) => return Err(Error::io("create", path, error)),
    };
    file.write_all(contents.as_bytes())
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
