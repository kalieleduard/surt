use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use super::worktree::{WorktreeCommit, resolve_head};
use super::write_failure;
use crate::runtime::RuntimeError;

#[derive(Clone, Debug)]
pub struct Delivery {
    pub(super) outcome: String,
    pub(super) outputs: Map<String, Value>,
    pub(super) files: Vec<(String, Vec<u8>)>,
    pub(super) final_json: FinalJson,
    pub(super) commit: Option<WorktreeCommit>,
}

#[derive(Clone, Debug)]
pub(super) enum FinalJson {
    Envelope,
    Text(Option<String>),
}

impl Delivery {
    pub fn new(outcome: &str) -> Delivery {
        Delivery {
            outcome: outcome.to_owned(),
            outputs: Map::new(),
            files: Vec::new(),
            final_json: FinalJson::Envelope,
            commit: None,
        }
    }

    pub fn output(self, name: &str, path: &str, contents: impl Into<Vec<u8>>) -> Delivery {
        self.output_without_file(name, path).file(path, contents)
    }

    pub fn output_without_file(mut self, name: &str, path: &str) -> Delivery {
        self.outputs
            .insert(name.to_owned(), Value::String(path.to_owned()));
        self
    }

    pub fn file(mut self, path: &str, contents: impl Into<Vec<u8>>) -> Delivery {
        self.files.push((path.to_owned(), contents.into()));
        self
    }

    pub fn final_json(mut self, text: Option<&str>) -> Delivery {
        self.final_json = FinalJson::Text(text.map(str::to_owned));
        self
    }

    pub(super) fn final_text(&self) -> Option<String> {
        match &self.final_json {
            FinalJson::Envelope => Some(self.envelope()),
            FinalJson::Text(text) => text.clone(),
        }
    }

    fn envelope(&self) -> String {
        let mut envelope = Map::new();
        envelope.insert("outcome".to_owned(), Value::String(self.outcome.clone()));
        envelope.insert("outputs".to_owned(), Value::Object(self.outputs.clone()));
        Value::Object(envelope).to_string()
    }

    pub(super) async fn write(
        &self,
        delivery_dir: &Path,
        worktree: &Path,
    ) -> Result<(), RuntimeError> {
        if let Some(commit) = &self.commit {
            commit.make(worktree).await?;
        }
        for (path, contents) in &self.files {
            let target = delivery_dir.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(write_failure)?;
            }
            fs::write(&target, resolve_head(contents, worktree).await?).map_err(write_failure)?;
        }
        Ok(())
    }
}
