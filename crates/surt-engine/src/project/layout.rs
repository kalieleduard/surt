use std::path::{Path, PathBuf};

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

fn slug(text: &str, max_chars: usize) -> String {
    let lowered = text.to_ascii_lowercase();
    let words: Vec<&str> = lowered
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let mut slug = String::new();
    for word in words {
        let separator = usize::from(!slug.is_empty());
        if slug.len() + separator + word.len() > max_chars {
            if slug.is_empty() {
                slug.push_str(&word[..max_chars]);
            }
            break;
        }
        if separator == 1 {
            slug.push('-');
        }
        slug.push_str(word);
    }
    slug
}

#[cfg(test)]
mod tests;
