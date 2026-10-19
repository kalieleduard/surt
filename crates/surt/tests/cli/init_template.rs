use std::fs;

use crate::init::state_of;
use crate::support::{REFUSED_EXIT_CODE, Sandbox};

const BUILDER_TOML: &str = ".surt/agents/builder/agent.toml";

const TEMPLATE_FILES: [&str; 25] = [
    ".surt/agents/builder/agent.toml",
    ".surt/agents/builder/instructions.md",
    ".surt/agents/builder/schemas/check-report.schema.json",
    ".surt/agents/builder/schemas/commit.schema.json",
    ".surt/agents/builder/schemas/conflict.schema.json",
    ".surt/agents/builder/schemas/feedback.schema.json",
    ".surt/agents/reviewer/agent.toml",
    ".surt/agents/reviewer/instructions.md",
    ".surt/agents/reviewer/schemas/check-report.schema.json",
    ".surt/agents/reviewer/schemas/commit.schema.json",
    ".surt/agents/reviewer/schemas/conflict.schema.json",
    ".surt/agents/reviewer/schemas/feedback.schema.json",
    ".surt/agents/shipper/agent.toml",
    ".surt/agents/shipper/instructions.md",
    ".surt/agents/shipper/schemas/base.schema.json",
    ".surt/agents/shipper/schemas/delivery.schema.json",
    ".surt/agents/shipper/schemas/integration.schema.json",
    ".surt/agents/shipper/schemas/manifest.schema.json",
    ".surt/agents/spec/agent.toml",
    ".surt/agents/spec/instructions.md",
    ".surt/agents/tasks/agent.toml",
    ".surt/agents/tasks/instructions.md",
    ".surt/agents/tasks/schemas/manifest.schema.json",
    ".surt/workflows/feature.toml",
    ".surt/workflows/task-cycle.toml",
];

fn visible_to_git(sandbox: &Sandbox) -> Vec<String> {
    let untracked = sandbox.git_output(
        &sandbox.repo(),
        &["status", "--porcelain", "--untracked-files=all"],
    );
    let mut paths: Vec<String> = untracked.lines().map(|line| line[3..].to_owned()).collect();
    paths.sort_unstable();
    paths
}

#[test]
fn the_feature_template_lists_every_file_it_creates() {
    let sandbox = Sandbox::with_repository();

    let run = sandbox.init();

    for path in TEMPLATE_FILES {
        assert_eq!(state_of(&run.stderr, path), Some("created"), "{path}");
        assert!(sandbox.repo().join(path).is_file(), "{path}");
    }
}

#[test]
fn init_with_the_feature_template_points_at_surt_run_feature() {
    let sandbox = Sandbox::with_repository();

    let run = sandbox.init();

    assert!(
        run.stderr.ends_with(
            "  next: surt run feature --input request=\"<what you want to build>\"
"
        ),
        "{}",
        run.stderr
    );
}

#[test]
fn config_workflows_and_agents_are_visible_to_git() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();

    let mut expected: Vec<String> = [".surt/.gitignore", ".surt/config.toml"]
        .iter()
        .chain(&TEMPLATE_FILES)
        .map(|path| (*path).to_owned())
        .collect();
    expected.sort_unstable();
    assert_eq!(visible_to_git(&sandbox), expected);
}

#[test]
fn rerunning_init_keeps_edited_files_and_restores_deleted_ones() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();
    let config = sandbox.dot_surt().join("config.toml");
    let builder = sandbox.repo().join(BUILDER_TOML);
    let instructions = sandbox.dot_surt().join("agents/reviewer/instructions.md");
    fs::write(&config, "max_attempts = 7\n").expect("an edited config");
    fs::write(&builder, "model = \"mine\"\n").expect("an edited agent");
    fs::remove_file(&instructions).expect("the instructions are removed");

    let rerun = sandbox.init();

    assert!(
        rerun.stderr.starts_with("✔ surt is already initialized in"),
        "{}",
        rerun.stderr
    );
    assert_eq!(state_of(&rerun.stderr, ".surt/config.toml"), Some("kept"));
    assert_eq!(state_of(&rerun.stderr, BUILDER_TOML), Some("kept"));
    assert_eq!(
        state_of(&rerun.stderr, ".surt/agents/reviewer/instructions.md"),
        Some("created")
    );
    assert_eq!(
        fs::read_to_string(&config).expect("readable"),
        "max_attempts = 7\n"
    );
    assert_eq!(
        fs::read_to_string(&builder).expect("readable"),
        "model = \"mine\"\n"
    );
    assert!(instructions.is_file());
}

#[test]
fn an_unknown_template_is_refused_with_the_list_of_templates() {
    let sandbox = Sandbox::with_repository();

    let run = sandbox.surt(&sandbox.repo(), &["init", "--template", "bugfix"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "✖ no template is named \"bugfix\"\n  templates: feature\n  fix: surt init --template feature\n"
    );
    assert!(!sandbox.dot_surt().exists());
}
