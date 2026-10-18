use std::fs;
use std::io;
use std::path::Path;

use surt_engine::project::LogSource;

use super::stream::Stream;
use crate::symbols::VERTICAL;

const STREAM: &str = "stream.jsonl";
const STDERR: &str = "stderr.log";
const OUTPUT: &str = "output.log";

pub(crate) struct Files {
    pub(crate) stream: String,
    pub(crate) stderr: String,
    pub(crate) output: String,
}

impl Files {
    pub(crate) fn read(directory: &Path) -> io::Result<Option<Files>> {
        if !directory.is_dir() {
            return Ok(None);
        }
        Ok(Some(Files {
            stream: contents(&directory.join(STREAM))?,
            stderr: contents(&directory.join(STDERR))?,
            output: contents(&directory.join(OUTPUT))?,
        }))
    }
}

fn contents(path: &Path) -> io::Result<String> {
    match fs::read(path) {
        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error),
    }
}

pub(crate) fn label(source: &LogSource) -> String {
    let step = format!("{}#{}", source.step, source.attempt);
    match &source.item {
        Some(item) => format!("{item} {step}"),
        None => step,
    }
}

pub(crate) fn prefixes(sources: &[&LogSource]) -> Vec<String> {
    if let [_] = sources {
        return vec![String::new()];
    }
    let labels: Vec<String> = sources.iter().map(|source| label(source)).collect();
    let width = labels
        .iter()
        .map(|label| label.chars().count())
        .max()
        .unwrap_or_default();
    labels
        .into_iter()
        .map(|label| format!("{label:<width$} {VERTICAL} "))
        .collect()
}

pub(crate) fn render(files: &Files, prefix: &str, raw: bool) -> String {
    if raw {
        return files.stream.clone();
    }
    let mut stream = Stream::default();
    let rendered = files.stream.lines().filter_map(|line| stream.render(line));
    let passed = files.stderr.lines().chain(files.output.lines());
    rendered
        .chain(passed.map(str::to_owned))
        .map(|line| format!("{prefix}{line}\n"))
        .collect()
}
