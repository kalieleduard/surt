use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use tempfile::TempDir;

use super::GlobalConfig;
use crate::project::config::read;
use crate::project::{Error, Refusal};

struct GlobalFile {
    dir: TempDir,
}

impl GlobalFile {
    fn new() -> Self {
        GlobalFile {
            dir: TempDir::new().expect("a temporary directory"),
        }
    }

    fn path(&self) -> PathBuf {
        self.dir.path().join("config.toml")
    }

    fn with(contents: &str) -> Self {
        let file = Self::new();
        fs::write(file.path(), contents).expect("the global config is written");
        file
    }

    fn load(&self) -> Result<GlobalConfig, Error> {
        read(&self.path())
    }

    fn refusal(&self) -> Refusal {
        match self.load() {
            Err(Error::Refused(refusal)) => refusal,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}

#[test]
fn a_missing_file_means_a_ten_second_grace_period() {
    let config = GlobalFile::new().load().expect("the defaults load");

    assert_eq!(config.stop_grace_period, Duration::from_secs(10));
    assert_eq!(config, GlobalConfig::default());
}

#[test]
fn an_empty_file_means_the_defaults() {
    assert_eq!(
        GlobalFile::with("").load().ok(),
        Some(GlobalConfig::default())
    );
}

#[test]
fn the_grace_period_is_read_in_the_check_timeout_format() {
    for (text, expected) in [
        ("500ms", Duration::from_millis(500)),
        ("30s", Duration::from_secs(30)),
        ("2m", Duration::from_secs(120)),
        ("1h", Duration::from_secs(3600)),
    ] {
        let file = GlobalFile::with(&format!("stop_grace_period = \"{text}\"\n"));

        let config = file.load().expect("the global config loads");

        assert_eq!(config.stop_grace_period, expected, "{text}");
    }
}

#[test]
fn an_unknown_key_is_refused_with_the_file_and_its_line() {
    let file = GlobalFile::with("stop_grace_period = \"10s\"\nstop_grace = \"5s\"\n");

    let Refusal::InvalidConfig { path, line, reason } = file.refusal() else {
        panic!("expected an invalid config");
    };

    assert_eq!(path, file.path());
    assert_eq!(line, Some(2));
    assert!(reason.contains("stop_grace"), "{reason}");
}

#[test]
fn a_project_key_in_the_global_file_is_refused_as_unknown() {
    let file = GlobalFile::with("max_concurrent_agents = 2\n");

    let Refusal::InvalidConfig { line, reason, .. } = file.refusal() else {
        panic!("expected an invalid config");
    };

    assert_eq!(line, Some(1));
    assert!(reason.contains("max_concurrent_agents"), "{reason}");
}

#[test]
fn a_zero_or_malformed_grace_period_is_refused_with_its_line() {
    for text in ["0s", "ten", "10", "10d", "-5s", "s"] {
        let file = GlobalFile::with(&format!(
            "# machine settings\nstop_grace_period = \"{text}\"\n"
        ));

        let Refusal::InvalidConfig { line, reason, .. } = file.refusal() else {
            panic!("expected an invalid config for {text}");
        };

        assert_eq!(line, Some(2), "{text}");
        assert!(reason.contains("stop_grace_period"), "{reason}");
    }
}

#[test]
fn a_grace_period_that_is_not_a_string_is_refused() {
    let file = GlobalFile::with("stop_grace_period = 10\n");

    assert!(matches!(
        file.refusal(),
        Refusal::InvalidConfig { line: Some(1), .. }
    ));
}
