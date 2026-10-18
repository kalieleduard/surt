use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const TAIL_LINES: usize = 200;

pub(super) fn read(log: &Path, offset: u64) -> io::Result<String> {
    let mut file = File::open(log)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(last_lines(&String::from_utf8_lossy(&bytes), TAIL_LINES))
}

fn last_lines(text: &str, count: usize) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    lines[lines.len().saturating_sub(count)..].concat()
}

#[cfg(test)]
mod tests;
