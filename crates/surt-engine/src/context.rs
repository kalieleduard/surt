use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::store::{Producer, StoredRevision};

const INPUTS_DIR: &str = "inputs";
const CONTEXT_FILE: &str = "context.md";

pub(crate) struct ContextInput<'a> {
    pub(crate) name: &'a str,
    pub(crate) revision: &'a StoredRevision,
}

pub(crate) struct PreviousIteration {
    pub(crate) number: u32,
    pub(crate) ended: Vec<(String, String)>,
}

pub(crate) struct Assembly<'a> {
    pub(crate) execution_dir: &'a Path,
    pub(crate) delivery_dir: &'a Path,
    pub(crate) previous: Option<PreviousIteration>,
    pub(crate) inputs: Vec<ContextInput<'a>>,
}

impl Assembly<'_> {
    pub(crate) fn write(&self) -> io::Result<String> {
        let mut text = String::new();
        if let Some(previous) = &self.previous {
            let ended: Vec<String> = previous
                .ended
                .iter()
                .map(|(step, ending)| format!("{step} = {ending}"))
                .collect();
            let _ = writeln!(text, "## Why this Iteration started");
            let _ = writeln!(
                text,
                "Iteration {} ended: {}",
                previous.number,
                ended.join(", ")
            );
        }
        let _ = writeln!(text, "## Inputs");
        for input in &self.inputs {
            let root = self.materialize(input)?;
            let _ = writeln!(
                text,
                "- {} rev {}   ({})   {}",
                input.name,
                input.revision.number,
                label(&input.revision.producer),
                root.display()
            );
        }
        let _ = writeln!(text, "## Delivery");
        let _ = writeln!(
            text,
            "Write every output under {}",
            self.delivery_dir.display()
        );
        fs::write(self.execution_dir.join(CONTEXT_FILE), &text)?;
        Ok(text)
    }

    fn materialize(&self, input: &ContextInput) -> io::Result<PathBuf> {
        let root = self.execution_dir.join(INPUTS_DIR).join(input.name);
        for (rel_path, bytes) in &input.revision.files {
            let target = root.join(rel_path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, bytes)?;
        }
        Ok(root)
    }
}

fn label(producer: &Producer) -> String {
    match producer {
        Producer::Step {
            key,
            iteration: Some(iteration),
        } => format!("iteration {iteration}, {key}"),
        Producer::Step {
            key,
            iteration: None,
        } => key.clone(),
        Producer::Human(action) => action.clone(),
    }
}
