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
        self.dot_surt()
            .join("runtime")
            .join("locks")
            .join("db.lock")
    }

    pub(crate) fn config(&self) -> PathBuf {
        self.dot_surt().join("config.toml")
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
