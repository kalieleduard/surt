use std::fs;

use crate::support::{REFUSED_EXIT_CODE, Sandbox};

#[test]
fn a_command_finds_the_project_from_a_subdirectory() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();

    let run = sandbox.surt(&sandbox.subdirectory("src/deep"), &["status"]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
}

#[test]
fn a_command_in_an_uninitialized_repository_points_at_surt_init() {
    let sandbox = Sandbox::with_repository();

    let run = sandbox.surt(&sandbox.subdirectory("src"), &["status"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr
            .starts_with("✖ not a surt project: no .surt/ in "),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("\n  run: surt init\n"),
        "{}",
        run.stderr
    );
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert!(!sandbox.dot_surt().exists());
}

#[test]
fn a_command_outside_any_repository_points_at_surt_init() {
    let sandbox = Sandbox::new();

    let run = sandbox.surt(&sandbox.repo(), &["status"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(run.stderr.contains("run: surt init"), "{}", run.stderr);
}

#[test]
fn a_command_inside_a_surt_managed_worktree_is_refused() {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    let managed = sandbox
        .home()
        .join(".surt/worktrees/repo-a1b2c3/20261017-add-login/A");
    fs::create_dir_all(managed.parent().expect("a parent")).expect("the worktrees directory");
    sandbox.git(
        &sandbox.repo(),
        &["worktree", "add", "--quiet", &managed.to_string_lossy()],
    );

    let run = sandbox.surt(&managed, &["status"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.contains("is inside a surt-managed worktree"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("  fix: cd "), "{}", run.stderr);
}

#[test]
fn an_unknown_config_key_is_refused_with_its_location() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();
    sandbox.write_config("max_attempts = 3\nmax_atempts = 4\n");

    let run = sandbox.surt(&sandbox.repo(), &["status"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.starts_with("✖ invalid configuration in "),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("config.toml:2"), "{}", run.stderr);
    assert!(run.stderr.contains("max_atempts"), "{}", run.stderr);
    assert!(run.stderr.contains("  fix: edit "), "{}", run.stderr);
}

#[test]
fn merge_into_base_fast_forward_only_is_refused() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();
    sandbox.write_config("[git]\nmerge_into_base = \"ff-only\"\n");

    let run = sandbox.surt(&sandbox.repo(), &["status"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr
            .starts_with("✖ merge_into_base = \"ff-only\" is not supported yet"),
        "{}",
        run.stderr
    );
}
