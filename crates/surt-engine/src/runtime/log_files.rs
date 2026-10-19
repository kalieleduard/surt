use std::path::{Path, PathBuf};

pub(crate) const STREAM_LOG: &str = "stream.jsonl";
pub(crate) const STDERR_LOG: &str = "stderr.log";
pub(crate) const OUTPUT_LOG: &str = "output.log";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogFiles {
    pub stream: PathBuf,
    pub stderr: PathBuf,
    pub output: PathBuf,
}

impl LogFiles {
    pub fn in_directory(directory: &Path) -> LogFiles {
        LogFiles {
            stream: directory.join(STREAM_LOG),
            stderr: directory.join(STDERR_LOG),
            output: directory.join(OUTPUT_LOG),
        }
    }
}
