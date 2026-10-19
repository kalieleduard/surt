use std::fs;

use super::fixtures::{
    ProjectDir, WRITER_AGENT, default_config, draft_catalog, invalid_file, new_run,
};
use crate::project::{DefinitionFile, Error, InvalidDefinition, Problem, Refusal};

const NOTES_OUTPUT: &str =
    "[outputs.notes]\nformat = \"json\"\njson_schema = \"schemas/notes.schema.json\"\n";

const LARGE_MODEL: &str = "model = \"fake-model-large\"";

fn large_writer() -> String {
    WRITER_AGENT.replace("model = \"fake-model\"", LARGE_MODEL)
}

async fn drafted(dir: &ProjectDir, edit: impl FnOnce(&ProjectDir)) {
    let project = dir.project(draft_catalog(), default_config()).await;
    edit(dir);
    project
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");
}

async fn refused(edit: impl FnOnce(&ProjectDir)) -> InvalidDefinition {
    let dir = ProjectDir::new();
    dir.project(draft_catalog(), default_config()).await;
    edit(&dir);
    invalid_file(&dir, "draft").await
}

#[tokio::test(flavor = "multi_thread")]
async fn the_agent_toml_read_from_dot_surt_is_frozen_whole_with_nothing_merged_in() {
    let dir = ProjectDir::new();
    let without_notes = large_writer().replace(NOTES_OUTPUT, "");

    drafted(&dir, |dir| {
        dir.write_agent_file("writer", "agent.toml", &without_notes);
    })
    .await;

    let frozen = dir.frozen_agent("writer").await;
    assert_eq!(
        frozen["profile"]["model"].as_str(),
        Some("fake-model-large")
    );
    assert!(frozen["outputs"].get("notes").is_none(), "{frozen}");
    assert_eq!(
        dir.frozen_agent_file("writer", "schemas/notes.schema.json")
            .await,
        None
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn crlf_and_lf_versions_of_an_agent_toml_freeze_to_the_same_identity() {
    let mut identities = Vec::new();
    for text in [large_writer(), large_writer().replace('\n', "\r\n")] {
        let dir = ProjectDir::new();
        drafted(&dir, |dir| {
            dir.write_agent_file("writer", "agent.toml", &text)
        })
        .await;
        let identity: Vec<(String,)> = dir
            .rows("SELECT sha256 FROM snapshot WHERE kind = 'agent' AND name = 'writer'")
            .await;
        identities.push(identity);
    }

    assert_eq!(identities[0], identities[1]);
}

#[tokio::test(flavor = "multi_thread")]
async fn broken_definitions_outside_the_closure_do_not_refuse_the_run() {
    let dir = ProjectDir::new();

    drafted(&dir, |dir| {
        dir.write_agent_file("broken", "agent.toml", "not = [toml");
        dir.write_agent_file("broken", "stray.txt", "");
        let dot_surt = dir.path().join(".surt");
        fs::write(dot_surt.join("workflows/broken.toml"), "schema = ").expect("a workflow");
        fs::write(dot_surt.join("agents/notes.txt"), "").expect("a stray file");
    })
    .await;

    let agents: Vec<(String,)> = dir
        .rows("SELECT name FROM snapshot WHERE kind = 'agent'")
        .await;
    assert_eq!(agents, [("writer".to_owned(),)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_agent_the_closure_uses_refuses_with_its_file_and_field() {
    let invalid = refused(|dir| {
        let huge = WRITER_AGENT.replace("effort = \"high\"", "effort = \"huge\"");
        dir.write_agent_file("writer", "agent.toml", &huge);
    })
    .await;

    assert_eq!(invalid.file.path(), ".surt/agents/writer/agent.toml");
    assert_eq!(invalid.field.as_deref(), Some("profile.effort"));
    assert!(
        matches!(&invalid.problem, Problem::Syntax { detail, .. } if detail.contains("huge")),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_workflow_file_whose_stem_is_not_its_name_refuses() {
    let dir = ProjectDir::new();
    dir.project(draft_catalog(), default_config()).await;
    let draft = dir.path().join(".surt/workflows/draft.toml");
    fs::rename(&draft, dir.path().join(".surt/workflows/other.toml")).expect("a renamed workflow");

    let invalid = invalid_file(&dir, "other").await;

    assert_eq!(invalid.file.path(), ".surt/workflows/other.toml");
    assert_eq!(
        (invalid.field.as_deref(), invalid.problem),
        (
            Some("name"),
            Problem::NameMismatch {
                found: "draft".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_without_its_instructions_or_its_agent_toml_refuses_naming_the_file() {
    for (removed, file) in [
        (
            "instructions.md",
            DefinitionFile::AgentFile {
                agent: "writer".to_owned(),
                path: "instructions.md".to_owned(),
            },
        ),
        ("agent.toml", DefinitionFile::Agent("writer".to_owned())),
    ] {
        let invalid = refused(|dir| {
            fs::remove_file(dir.path().join(".surt/agents/writer").join(removed))
                .expect("the file is removed");
        })
        .await;

        assert_eq!(
            (invalid.file, invalid.problem),
            (file, Problem::MissingFile)
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_that_is_not_utf8_refuses_naming_it() {
    let invalid = refused(|dir| {
        fs::write(
            dir.path().join(".surt/agents/writer/instructions.md"),
            [0xff, 0xfe],
        )
        .expect("a binary file");
    })
    .await;

    assert_eq!(invalid.file.path(), ".surt/agents/writer/instructions.md");
    assert!(matches!(
        invalid.problem,
        Problem::Syntax { line: None, .. }
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_workflow_refuses_listing_the_workflows_found_in_dot_surt() {
    for (catalog, known) in [
        (draft_catalog(), vec!["draft".to_owned()]),
        (Default::default(), vec![]),
    ] {
        let dir = ProjectDir::new();
        let project = dir.project(catalog, default_config()).await;

        let result = project.create_run(new_run("missing")).await;

        dir.assert_nothing_written().await;
        assert!(
            matches!(&result, Err(Error::Refused(Refusal::UnknownWorkflow { name, known: found })) if name == "missing" && *found == known),
            "{result:?}"
        );
    }
}
