use std::fs;

use crate::support::{REFUSED_EXIT_CODE, Sandbox};

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
        run.stderr
            .contains("next: surt feature \"<what you want to build>\""),
        "{}",
        run.stderr
    );
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    let gitignore = fs::read_to_string(sandbox.dot_surt().join(".gitignore")).expect("readable");
    assert!(gitignore.contains("!config.toml"));
}

#[test]
fn only_config_and_agents_are_visible_to_git() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();
    fs::create_dir_all(sandbox.dot_surt().join("agents/builder")).expect("an agents directory");
    fs::write(sandbox.dot_surt().join("agents/builder/agent.toml"), "").expect("an override");

    let untracked = sandbox.git_output(
        &sandbox.repo(),
        &["status", "--porcelain", "--untracked-files=all"],
    );

    let mut paths: Vec<&str> = untracked.lines().map(|line| &line[3..]).collect();
    paths.sort_unstable();
    assert_eq!(
        paths,
        [
            ".surt/.gitignore",
            ".surt/agents/builder/agent.toml",
            ".surt/config.toml"
        ]
    );
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
fn rerunning_init_keeps_config_and_agents_and_reports_what_exists() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();
    let config = sandbox.dot_surt().join("config.toml");
    let agent = sandbox.dot_surt().join("agents/builder/agent.toml");
    fs::write(&config, "max_attempts = 7\n").expect("an edited config");
    fs::create_dir_all(agent.parent().expect("a parent")).expect("an agents directory");
    fs::write(&agent, "model = \"mine\"\n").expect("an override");

    let rerun = sandbox.init();

    assert!(
        rerun.stderr.starts_with("✔ surt is already initialized in"),
        "{}",
        rerun.stderr
    );
    assert!(rerun.stderr.contains("(exists, kept)"), "{}", rerun.stderr);
    assert_eq!(
        fs::read_to_string(&config).expect("readable"),
        "max_attempts = 7\n"
    );
    assert_eq!(
        fs::read_to_string(&agent).expect("readable"),
        "model = \"mine\"\n"
    );
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
