use super::super::CleanupConfig;
use super::ConfigFile;
use crate::project::{OnSuccess, Refusal};

#[test]
fn the_cleanup_policy_on_success_is_absent_by_default() {
    let config = ConfigFile::with("max_attempts = 3\n")
        .load()
        .expect("the config loads");

    assert_eq!(config.cleanup, CleanupConfig { on_success: None });
}

#[test]
fn a_succeeded_run_may_be_cleaned_or_kept() {
    for (text, policy) in [("clean", OnSuccess::Clean), ("keep", OnSuccess::Keep)] {
        let file = ConfigFile::with(&format!("[cleanup]\non_success = \"{text}\"\n"));

        let config = file.load().expect("the config loads");

        assert_eq!(config.cleanup.on_success, Some(policy), "{text}");
    }
}

#[test]
fn an_unknown_cleanup_policy_is_refused_with_its_line() {
    let file = ConfigFile::with("max_attempts = 3\n\n[cleanup]\non_success = \"keeep\"\n");

    let Refusal::InvalidConfig { path, line, reason } = file.refusal() else {
        panic!("expected an invalid config");
    };

    assert_eq!(path, file.path());
    assert_eq!(line, Some(4));
    assert!(reason.contains("keeep"), "{reason}");
}
