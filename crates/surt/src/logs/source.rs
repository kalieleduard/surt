use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use surt_engine::project::{LogFiles, LogSource};

use super::stream::Stream;
use crate::symbols::VERTICAL;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Rendered,
    Raw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pass {
    Ongoing,
    Final,
}

#[derive(Default)]
pub(crate) struct PerFile<T> {
    pub(crate) stream: T,
    pub(crate) stderr: T,
    pub(crate) output: T,
}

pub(crate) type Files = PerFile<String>;
pub(crate) type Offsets = PerFile<u64>;

impl<T> PerFile<T> {
    fn try_map<U>(self, mut each: impl FnMut(T) -> io::Result<U>) -> io::Result<PerFile<U>> {
        Ok(PerFile {
            stream: each(self.stream)?,
            stderr: each(self.stderr)?,
            output: each(self.output)?,
        })
    }

    fn zip<U>(self, other: PerFile<U>) -> PerFile<(T, U)> {
        PerFile {
            stream: (self.stream, other.stream),
            stderr: (self.stderr, other.stderr),
            output: (self.output, other.output),
        }
    }

    fn as_mut(&mut self) -> PerFile<&mut T> {
        PerFile {
            stream: &mut self.stream,
            stderr: &mut self.stderr,
            output: &mut self.output,
        }
    }
}

fn paths(files: &LogFiles) -> PerFile<&Path> {
    PerFile {
        stream: &files.stream,
        stderr: &files.stderr,
        output: &files.output,
    }
}

impl Files {
    pub(crate) fn read(source: &LogSource) -> io::Result<Option<Files>> {
        if !source.directory.is_dir() {
            return Ok(None);
        }
        paths(&source.files).try_map(contents).map(Some)
    }

    pub(crate) fn appended(
        files: &LogFiles,
        offsets: &mut Offsets,
        pass: Pass,
    ) -> io::Result<Files> {
        paths(files)
            .zip(offsets.as_mut())
            .try_map(|(path, offset)| appended(path, offset, pass))
    }
}

fn appended(path: &Path, offset: &mut u64, pass: Pass) -> io::Result<String> {
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
    bytes.truncate(match pass {
        Pass::Final => bytes.len(),
        Pass::Ongoing => lines,
    });
    *offset += u64::try_from(bytes.len()).unwrap_or(u64::MAX);
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

pub(crate) fn render(files: &Files, prefix: &str, format: Format) -> String {
    render_with(&mut Stream::default(), files, prefix, format)
}

pub(crate) fn render_with(
    stream: &mut Stream,
    files: &Files,
    prefix: &str,
    format: Format,
) -> String {
    if format == Format::Raw {
        return files.stream.clone();
    }
    let rendered = files.stream.lines().filter_map(|line| stream.render(line));
    let passed = files.stderr.lines().chain(files.output.lines());
    rendered
        .chain(passed.map(str::to_owned))
        .map(|line| format!("{prefix}{line}\n"))
        .collect()
}
