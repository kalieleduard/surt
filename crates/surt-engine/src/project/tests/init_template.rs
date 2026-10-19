use std::fs;

use super::Sandbox;
use crate::project::{Error, FileState, InitFile};

const TEMPLATE: [(&str, &[u8]); 2] = [
    ("workflows/flow.toml", b"schema = 1\n"),
    ("agents/writer/instructions.md", b"Write.\n"),
];

fn template_states(files: &[(InitFile, FileState)]) -> Vec<(String, FileState)> {
    files
        .iter()
        .filter_map(|(file, state)| match file {
            InitFile::Template(path) => Some((path.clone(), *state)),
            _ => None,
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn init_writes_the_template_files_under_dot_surt_and_reports_them_created() {
    let sandbox = Sandbox::with_repository();

    let initialized = sandbox
        .init_with(&sandbox.repo(), &TEMPLATE)
        .await
        .expect("init succeeds");

    assert_eq!(
        template_states(&initialized.files),
        [
            ("workflows/flow.toml".to_owned(), FileState::Created),
            (
                "agents/writer/instructions.md".to_owned(),
                FileState::Created
            ),
        ]
    );
    let written = fs::read(sandbox.repo().join(".surt/agents/writer/instructions.md"));
    assert_eq!(written.expect("the instructions"), b"Write.\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn rerunning_init_keeps_edited_template_files_and_restores_deleted_ones() {
    let sandbox = Sandbox::with_repository();
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    sandbox
        .init_with(&sandbox.repo(), &TEMPLATE)
        .await
        .expect("the template is written");
    let workflow = sandbox.repo().join(".surt/workflows/flow.toml");
    let instructions = sandbox.repo().join(".surt/agents/writer/instructions.md");
    fs::write(&workflow, "edited\n").expect("an edited workflow");
    fs::remove_file(&instructions).expect("the instructions are removed");

    let rerun = sandbox
        .init_with(&sandbox.repo(), &TEMPLATE)
        .await
        .expect("rerun succeeds");

    assert!(rerun.was_already_initialized());
    assert_eq!(
        template_states(&rerun.files),
        [
            ("workflows/flow.toml".to_owned(), FileState::Kept),
            (
                "agents/writer/instructions.md".to_owned(),
                FileState::Created
            ),
        ]
    );
    assert_eq!(fs::read_to_string(&workflow).expect("readable"), "edited\n");
    assert!(instructions.is_file());
}

#[tokio::test(flavor = "multi_thread")]
async fn plain_init_writes_no_definitions() {
    let sandbox = Sandbox::with_repository();

    let initialized = sandbox.init(&sandbox.repo()).await.expect("init succeeds");

    assert_eq!(template_states(&initialized.files), []);
    assert!(!sandbox.repo().join(".surt/workflows").exists());
    assert!(!sandbox.repo().join(".surt/agents").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_template_path_leaving_dot_surt_is_not_written() {
    let sandbox = Sandbox::with_repository();

    let result = sandbox
        .init_with(&sandbox.repo(), &[("../escaped.toml", b"x")])
        .await;

    assert!(matches!(result, Err(Error::Internal(_))), "{result:?}");
    assert!(!sandbox.repo().join("escaped.toml").exists());
}
