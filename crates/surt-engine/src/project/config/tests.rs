use std::fs;
use std::num::NonZeroU32;
use std::path::PathBuf;

use tempfile::TempDir;

use super::{Check, Config, MergeIntoBase, TEMPLATE, load};
use crate::project::{Error, Refusal};

mod cleanup;

struct ConfigFile {
    dir: TempDir,
}

impl ConfigFile {
    fn new() -> Self {
        ConfigFile {
            dir: TempDir::new().expect("a temporary directory"),
        }
    }

    fn path(&self) -> PathBuf {
        self.dir.path().join("config.toml")
    }

    fn with(contents: &str) -> Self {
        let file = Self::new();
        fs::write(file.path(), contents).expect("the config is written");
        file
    }

    fn load(&self) -> Result<Config, Error> {
        load(&self.path())
    }

    fn refusal(&self) -> Refusal {
        match self.load() {
            Err(Error::Refused(refusal)) => refusal,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}

fn nonzero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("a nonzero value")
}

#[test]
fn the_generated_template_holds_exactly_the_documented_defaults() {
    let config = ConfigFile::with(TEMPLATE)
        .load()
        .expect("the template loads");

    assert_eq!(config.max_concurrent_agents, nonzero(3));
    assert_eq!(config.max_attempts, nonzero(3));
    assert_eq!(config.max_iterations, nonzero(5));
    assert_eq!(config.git.merge_into_base, MergeIntoBase::Never);
    assert!(config.checks.is_empty());
    assert_eq!(config.cleanup.on_success, None);
    assert_eq!(config, Config::default());
}

#[test]
fn a_missing_file_means_the_defaults() {
    assert_eq!(ConfigFile::new().load().ok(), Some(Config::default()));
}

#[test]
fn checks_are_read_in_order_with_an_optional_timeout() {
    let file = ConfigFile::with(
        "[[checks]]\nname = \"test\"\nrun = [\"cargo\", \"test\"]\ntimeout = \"10m\"\n\n[[checks]]\nname = \"lint\"\nrun = [\"cargo\", \"clippy\"]\n",
    );

    let config = file.load().expect("the config loads");

    assert_eq!(
        config.checks,
        [
            Check {
                name: "test".to_owned(),
                run: vec!["cargo".to_owned(), "test".to_owned()],
                timeout: Some("10m".to_owned()),
            },
            Check {
                name: "lint".to_owned(),
                run: vec!["cargo".to_owned(), "clippy".to_owned()],
                timeout: None,
            },
        ]
    );
}

#[test]
fn a_check_timeout_in_milliseconds_seconds_minutes_or_hours_is_accepted() {
    for timeout in ["250ms", "30s", "10m", "2h"] {
        let file = ConfigFile::with(&format!(
            "[[checks]]\nname = \"test\"\nrun = [\"cargo\", \"test\"]\ntimeout = \"{timeout}\"\n"
        ));

        let config = file.load().expect("the config loads");

        assert_eq!(config.checks[0].timeout.as_deref(), Some(timeout));
    }
}

#[test]
fn a_check_timeout_without_a_known_unit_is_refused_with_its_line() {
    for timeout in ["10", "ten minutes", "10d", "m", "0s"] {
        let file = ConfigFile::with(&format!(
            "[[checks]]\nname = \"test\"\nrun = [\"cargo\", \"test\"]\ntimeout = \"{timeout}\"\n"
        ));

        let Refusal::InvalidConfig { line, reason, .. } = file.refusal() else {
            panic!("expected an invalid config for {timeout}");
        };

        assert_eq!(line, Some(4), "{timeout}");
        assert!(reason.contains("timeout"), "{reason}");
    }
}

#[test]
fn a_check_without_a_program_to_run_is_refused() {
    let file = ConfigFile::with("[[checks]]\nname = \"test\"\nrun = []\n");

    let Refusal::InvalidConfig { line, reason, .. } = file.refusal() else {
        panic!("expected an invalid config");
    };

    assert_eq!(line, Some(3));
    assert!(reason.contains("run"), "{reason}");
}

#[test]
fn a_check_name_that_is_not_an_identifier_is_refused() {
    let file = ConfigFile::with("[[checks]]\nname = \"Unit Tests\"\nrun = [\"cargo\", \"test\"]\n");

    let Refusal::InvalidConfig { line, reason, .. } = file.refusal() else {
        panic!("expected an invalid config");
    };

    assert_eq!(line, Some(2));
    assert!(reason.contains("Unit Tests"), "{reason}");
}

#[test]
fn an_unknown_top_level_key_is_refused_with_its_line() {
    let file = ConfigFile::with("max_attempts = 3\nmax_atempts = 4\n");

    let Refusal::InvalidConfig { path, line, reason } = file.refusal() else {
        panic!("expected an invalid config");
    };

    assert_eq!(path, file.path());
    assert_eq!(line, Some(2));
    assert!(reason.contains("max_atempts"), "{reason}");
}

#[test]
fn an_unknown_key_inside_a_table_is_refused() {
    let file = ConfigFile::with("[git]\nmerge_into_base = \"never\"\npush = true\n");

    assert!(matches!(
        file.refusal(),
        Refusal::InvalidConfig { line: Some(3), .. }
    ));
}

#[test]
fn a_zero_limit_is_refused() {
    let file = ConfigFile::with("max_attempts = 0\n");

    assert!(matches!(
        file.refusal(),
        Refusal::InvalidConfig { line: Some(1), .. }
    ));
}

#[test]
fn merging_into_base_fast_forward_only_parses() {
    let config = ConfigFile::with("[git]\nmerge_into_base = \"ff-only\"\n")
        .load()
        .expect("ff-only loads");

    assert_eq!(config.git.merge_into_base, MergeIntoBase::FfOnly);
}

#[test]
fn an_unknown_merge_mode_is_an_invalid_config() {
    let file = ConfigFile::with("[git]\nmerge_into_base = \"always\"\n");

    assert!(matches!(
        file.refusal(),
        Refusal::InvalidConfig { line: Some(2), .. }
    ));
}
