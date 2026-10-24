use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::store::{Producer, StoredRevision};

const INPUTS_DIR: &str = "inputs";
const CONTEXT_FILE: &str = "context.md";
const CONTINUED_FILE: &str = "context-continued.md";

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
    pub(crate) references: Vec<(&'a str, &'a str)>,
}

impl Assembly<'_> {
    pub(crate) fn write(&self) -> io::Result<String> {
        for input in &self.inputs {
            self.materialize(input)?;
        }
        for (rel_path, text) in &self.references {
            write_creating(&self.execution_dir.join(rel_path), text.as_bytes())?;
        }
        self.write_file(CONTEXT_FILE, |_| true)
    }

    pub(crate) fn write_continued(&self, changed: &[&str]) -> io::Result<String> {
        self.write_file(CONTINUED_FILE, |input| changed.contains(&input.name))
    }

    fn write_file(&self, name: &str, kept: impl Fn(&ContextInput) -> bool) -> io::Result<String> {
        let mut text = self.previous.as_ref().map_or_else(String::new, why);
        let _ = writeln!(text, "## Inputs");
        for input in self.inputs.iter().filter(|input| kept(input)) {
            let _ = writeln!(
                text,
                "- {} rev {}   ({})   {}",
                input.name,
                input.revision.number,
                label(&input.revision.producer),
                self.root(input).display()
            );
        }
        if !self.references.is_empty() {
            let _ = writeln!(text, "## References");
            for (rel_path, _) in &self.references {
                let _ = writeln!(text, "- {}", self.execution_dir.join(rel_path).display());
            }
        }
        let _ = writeln!(text, "## Delivery");
        let _ = writeln!(
            text,
            "Write every output under {}",
            self.delivery_dir.display()
        );
        fs::write(self.execution_dir.join(name), &text)?;
        Ok(text)
    }

    fn root(&self, input: &ContextInput) -> PathBuf {
        self.execution_dir.join(INPUTS_DIR).join(input.name)
    }

    fn materialize(&self, input: &ContextInput) -> io::Result<()> {
        let root = self.root(input);
        for (rel_path, bytes) in &input.revision.files {
            write_creating(&root.join(rel_path), bytes)?;
        }
        Ok(())
    }
}

fn write_creating(target: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, bytes)
}

fn why(previous: &PreviousIteration) -> String {
    let ended: Vec<String> = previous
        .ended
        .iter()
        .map(|(step, ending)| format!("{step} = {ending}"))
        .collect();
    format!(
        "## Why this Iteration started\nIteration {} ended: {}\n",
        previous.number,
        ended.join(", ")
    )
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
