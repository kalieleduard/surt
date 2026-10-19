use super::fixtures::{Catalog, ProjectDir, agent, default_config, input, new_run};
use crate::project::{Error, Project};

const INPUTS_WORKFLOW: &str = r#"
schema = 1
name = "inputs"

[inputs.request]
format = "text"
description = "what to build"

[inputs.notes]
format = "markdown"
optional = true

[inputs.settings]
format = "json"
optional = true

[inputs.base]
format = "git_ref"
default = "HEAD"
description = "where to start"

[subject]
from = "request"

[steps.read]
kind = "agent"
agent = "reader"
"#;

const READER: &str = r#"
schema = 1
id = "reader"

[inputs.request]
format = "text"

[outputs.notes]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const BARE_WORKFLOW: &str = r#"
schema = 1
name = "bare"

[steps.read]
kind = "agent"
agent = "idle"
"#;

const IDLE: &str = r#"
schema = 1
id = "idle"

[outputs.notes]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

pub(super) async fn inputs_project(dir: &ProjectDir) -> Project {
    let catalog = Catalog::new()
        .workflow("inputs", INPUTS_WORKFLOW)
        .workflow("bare", BARE_WORKFLOW)
        .agent("reader", agent(READER))
        .agent("idle", agent(IDLE));
    dir.project(catalog, default_config()).await
}

async fn created_with(
    dir: &ProjectDir,
    inputs: &[(&str, &str)],
) -> Result<Vec<(String, String, String)>, Error> {
    let project = inputs_project(dir).await;
    let mut run = new_run("inputs");
    run.inputs = inputs
        .iter()
        .map(|(name, value)| input(name, value))
        .collect();
    project.create_run(run).await?;
    Ok(dir
        .rows("SELECT a.name, f.rel_path, CAST(b.bytes AS TEXT) FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id JOIN revision_file f ON f.revision_id = r.id JOIN blob b ON b.sha256 = f.blob_sha256 ORDER BY a.name")
        .await)
}

fn artifact(name: &str, rel_path: &str, text: &str) -> (String, String, String) {
    (name.to_owned(), rel_path.to_owned(), text.to_owned())
}

#[tokio::test(flavor = "multi_thread")]
async fn each_input_format_becomes_an_initial_artifact_named_after_the_input() {
    let dir = ProjectDir::new();

    let artifacts = created_with(
        &dir,
        &[
            ("request", "add login"),
            ("notes", "# Notes\n"),
            ("settings", r#"{"retries": 2}"#),
        ],
    )
    .await
    .expect("the run is created");

    let base = format!(r#"{{"ref":"main","sha":"{}"}}"#, dir.head());
    assert_eq!(
        artifacts,
        [
            artifact("base", "base.json", &base),
            artifact("notes", "notes.md", "# Notes\n"),
            artifact("request", "request.txt", "add login"),
            artifact("settings", "settings.json", r#"{"retries": 2}"#),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_git_ref_given_as_a_commit_is_detached_and_resolved_to_its_full_sha() {
    let dir = ProjectDir::new();
    dir.ensure_repository();
    let head = dir.head();

    let artifacts = created_with(&dir, &[("request", "add login"), ("base", &head[..7])])
        .await
        .expect("the run is created");

    let base = format!(r#"{{"ref":null,"sha":"{head}"}}"#);
    assert_eq!(artifacts[0], artifact("base", "base.json", &base));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_created_run_reports_each_resolved_git_ref() {
    let dir = ProjectDir::new();
    let project = inputs_project(&dir).await;

    let created = project
        .create_run(new_run("inputs"))
        .await
        .expect("the run is created");

    assert_eq!(created.git_refs.len(), 1);
    assert_eq!(created.git_refs[0].0, "base");
    assert_eq!(created.git_refs[0].1.branch.as_deref(), Some("main"));
    assert_eq!(created.git_refs[0].1.sha, dir.head());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_workflow_without_inputs_runs_without_any() {
    let dir = ProjectDir::new();
    let project = inputs_project(&dir).await;
    let mut run = new_run("bare");
    run.inputs.clear();

    let created = project.create_run(run).await;

    assert!(created.is_ok(), "{created:?}");
    assert_eq!(project.subject_input("bare", &[]).ok(), Some(None));
    let artifacts: Vec<(i64,)> = dir.rows("SELECT count(*) FROM artifact").await;
    assert_eq!(artifacts, [(0,)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_subject_input_is_the_value_given_for_the_input_subject_from_names() {
    let dir = ProjectDir::new();
    let project = inputs_project(&dir).await;
    let given = [input("notes", "# Notes"), input("request", "Add Login")];

    let subject = project.subject_input("inputs", &given);

    assert_eq!(subject.ok(), Some(Some("Add Login")));
}
