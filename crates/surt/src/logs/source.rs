use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
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

#[derive(Default)]
pub(crate) struct Offsets {
    stream: u64,
    stderr: u64,
    output: u64,
}

impl Files {
    pub(crate) fn appended(
        directory: &Path,
        offsets: &mut Offsets,
        whole: bool,
    ) -> io::Result<Files> {
        Ok(Files {
            stream: appended(&directory.join(STREAM), &mut offsets.stream, whole)?,
            stderr: appended(&directory.join(STDERR), &mut offsets.stderr, whole)?,
            output: appended(&directory.join(OUTPUT), &mut offsets.output, whole)?,
        })
    }
}

fn appended(path: &Path, offset: &mut u64, whole: bool) -> io::Result<String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(String::new()),
        Err(error) => return Err(error),
    };
    file.seek(SeekFrom::Start(*offset))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let lines = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |last| last + 1);
    bytes.truncate(if whole { bytes.len() } else { lines });
    *offset += u64::try_from(bytes.len()).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
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
    aligned(sources)
}

pub(crate) fn aligned(sources: &[&LogSource]) -> Vec<String> {
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
    render_with(&mut Stream::default(), files, prefix, raw)
}

pub(crate) fn render_with(stream: &mut Stream, files: &Files, prefix: &str, raw: bool) -> String {
    if raw {
        return files.stream.clone();
    }
    let rendered = files.stream.lines().filter_map(|line| stream.render(line));
    let passed = files.stderr.lines().chain(files.output.lines());
    rendered
        .chain(passed.map(str::to_owned))
        .map(|line| format!("{prefix}{line}\n"))
        .collect()
}
