use std::fs;

use crate::support::{REFUSED_EXIT_CODE, Sandbox};

pub(crate) fn state_of<'a>(stderr: &'a str, path: &str) -> Option<&'a str> {
    stderr
        .lines()
        .find_map(|line| line.trim().strip_prefix(path))
        .map(str::trim)
}

#[test]
fn init_from_a_subdirectory_creates_dot_surt_at_the_toplevel() {
    let sandbox = Sandbox::with_repository();
    let subdirectory = sandbox.subdirectory("src/deep");

    let run = sandbox.surt(&subdirectory, &["init"]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(sandbox.dot_surt().join("surt.db").is_file());
    assert!(sandbox.dot_surt().join("config.toml").is_file());
    assert!(!subdirectory.join(".surt").exists());
    assert!(
        run.stderr.starts_with("✔ initialized surt in"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("toplevel"), "{}", run.stderr);
    assert!(
        run.stderr.contains("identity tester@example.com ✔"),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr.contains(
            "next: add a workflow under .surt/workflows/
"
        ),
        "{}",
        run.stderr
    );
    assert!(run.stdout.is_empty(), "{}", run.stdout);
}

#[test]
fn plain_init_writes_no_workflow_and_no_agent() {
    let sandbox = Sandbox::with_repository();

    let run = sandbox.surt(&sandbox.repo(), &["init"]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(state_of(&run.stderr, ".surt/config.toml"), Some("created"));
    assert!(
        !run.stderr.contains("workflows/feature.toml"),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr
            .ends_with("  next: add a workflow under .surt/workflows/\n"),
        "{}",
        run.stderr
    );
    assert!(!sandbox.dot_surt().join("workflows").exists());
    assert!(!sandbox.dot_surt().join("agents").exists());
}

#[test]
fn the_generated_config_is_commented_and_holds_the_defaults() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();

    let config = fs::read_to_string(sandbox.dot_surt().join("config.toml")).expect("readable");

    for expected in [
        "max_concurrent_agents = 3",
        "max_attempts = 3",
        "max_iterations = 5",
        "merge_into_base = \"never\"",
        "# [[checks]]",
    ] {
        assert!(
            config.contains(expected),
            "{expected} missing from:\n{config}"
        );
    }
}

#[test]
fn init_without_a_git_identity_is_refused_with_a_fix() {
    let sandbox = Sandbox::with_repository();
    sandbox.without_identity();

    let run = sandbox.surt(&sandbox.repo(), &["init"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.starts_with("✖ git has no user identity"),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("  fix: git config --global user.email"),
        "{}",
        run.stderr
    );
    assert!(!sandbox.dot_surt().exists());
}

#[test]
fn init_outside_a_git_repository_is_refused() {
    let sandbox = Sandbox::new();

    let run = sandbox.surt(&sandbox.repo(), &["init"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.contains("is not inside a git repository"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("  fix: git init"), "{}", run.stderr);
}
