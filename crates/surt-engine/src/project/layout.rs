use std::path::{Path, PathBuf};

use super::naming::slug;

const DOT_SURT: &str = ".surt";
const DEFAULT_PROJECT_NAME: &str = "project";
const MAX_PROJECT_NAME_CHARS: usize = 24;

pub(crate) struct Layout {
    root: PathBuf,
}

impl Layout {
    pub(crate) fn new(root: PathBuf) -> Layout {
        Layout { root }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn dot_surt(&self) -> PathBuf {
        self.root.join(DOT_SURT)
    }

    pub(crate) fn database(&self) -> PathBuf {
        self.dot_surt().join("surt.db")
    }

    pub(crate) fn database_lock(&self) -> PathBuf {
        self.locks().join("db.lock")
    }

    pub(crate) fn git_lock(&self) -> PathBuf {
        self.locks().join("git.lock")
    }

    pub(crate) fn run_lock(&self, run: &str) -> PathBuf {
        self.locks().join(format!("{run}.lock"))
    }

    pub(crate) fn slot(&self, index: usize) -> PathBuf {
        self.runtime()
            .join("slots")
            .join(format!("agent-{index}.lock"))
    }

    pub(crate) fn execution(&self, execution: &str) -> PathBuf {
        self.runtime().join("executions").join(execution)
    }

    pub(crate) fn execution_logs(&self, subject: &str, execution: &str) -> PathBuf {
        self.logs(subject).join(execution)
    }

    pub(crate) fn logs(&self, subject: &str) -> PathBuf {
        self.dot_surt().join("logs").join(subject)
    }

    pub(crate) fn working_copy(&self, subject: &str) -> PathBuf {
        self.dot_surt().join("features").join(subject)
    }

    fn runtime(&self) -> PathBuf {
        self.dot_surt().join("runtime")
    }

    fn locks(&self) -> PathBuf {
        self.runtime().join("locks")
    }

    pub(crate) fn config(&self) -> PathBuf {
        self.dot_surt().join("config.toml")
    }

    pub(crate) fn agents(&self) -> PathBuf {
        self.dot_surt().join("agents")
    }

    pub(crate) fn gitignore(&self) -> PathBuf {
        self.dot_surt().join(".gitignore")
    }

    pub(crate) fn project_name(&self) -> String {
        let directory = self
            .root
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();
        let name = slug(&directory, MAX_PROJECT_NAME_CHARS);
        if name.is_empty() {
            return DEFAULT_PROJECT_NAME.to_owned();
        }
        name
    }
}

#[cfg(test)]
mod tests;
