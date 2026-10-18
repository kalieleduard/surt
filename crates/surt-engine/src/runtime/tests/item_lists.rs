use super::fixtures::Execution;
use super::planner::{TWO_TASKS, file_rejection, item, list, manifest, planner_contract, problem};
use crate::artifacts::{DeliveryRejection, ItemListProblem};
use crate::runtime::{Behavior, Verdict};

#[tokio::test]
async fn an_item_list_revision_holds_the_index_and_every_referenced_file() {
    let execution = Execution::with_contract(planner_contract());

    let verdict = execution
        .run(Behavior::Deliver(
            manifest(TWO_TASKS).file("scratch/notes.md", "unreferenced"),
        ))
        .await
        .expect("the fake completes");

    let Verdict::Accepted(delivery) = verdict else {
        panic!("expected an accepted delivery, got {verdict:?}");
    };
    let files: Vec<(&str, &[u8])> = delivery.outputs["manifest"]
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.bytes.as_slice()))
        .collect();
    assert_eq!(
        files,
        [
            ("manifest.json", TWO_TASKS.as_bytes()),
            ("tasks/A.md", b"# A\n".as_slice()),
            ("tasks/B.md", b"# B\n".as_slice()),
        ]
    );
}

#[tokio::test]
async fn an_index_that_is_not_a_list_is_rejected() {
    let problem = problem(manifest(r#"{ "key": "A" }"#)).await;

    assert_eq!(problem, ItemListProblem::NotAList);
}

#[tokio::test]
async fn an_item_without_a_key_or_needs_is_rejected() {
    let index = r#"[{ "key": "A", "needs": [], "title": "t", "files": { "task": "tasks/A.md" } }, { "key": "B", "title": "t" }]"#;

    let problem = problem(manifest(index)).await;

    assert_eq!(problem, ItemListProblem::MalformedItem { index: 1 });
}

#[tokio::test]
async fn keys_that_are_not_portable_are_rejected() {
    let too_long = "K".repeat(33);
    for key in ["a b", "-A", "", "tâche", too_long.as_str()] {
        let index = list(&[item(key, &[], "tasks/A.md")]);

        let problem = problem(manifest(&index)).await;

        assert_eq!(
            problem,
            ItemListProblem::InvalidKey {
                key: key.to_owned()
            },
            "{key:?}"
        );
    }
}

#[tokio::test]
async fn a_windows_device_name_is_not_a_key() {
    for key in ["con", "Nul", "COM0", "lpt9"] {
        let index = list(&[item(key, &[], "tasks/A.md")]);

        let problem = problem(manifest(&index)).await;

        assert_eq!(
            problem,
            ItemListProblem::ReservedKey {
                key: key.to_owned()
            }
        );
    }
}

#[tokio::test]
async fn keys_are_unique_ignoring_case() {
    let index = list(&[item("a", &[], "tasks/A.md"), item("A", &[], "tasks/B.md")]);

    let problem = problem(manifest(&index)).await;

    assert_eq!(
        problem,
        ItemListProblem::DuplicateKey {
            key: "A".to_owned()
        }
    );
}

#[tokio::test]
async fn needs_must_name_keys_of_the_same_list() {
    let index = list(&[item("A", &["Z"], "tasks/A.md")]);

    let problem = problem(manifest(&index)).await;

    assert_eq!(
        problem,
        ItemListProblem::UnknownNeed {
            key: "A".to_owned(),
            need: "Z".to_owned()
        }
    );
}

#[tokio::test]
async fn a_cycle_in_needs_is_rejected_with_its_path() {
    let index = list(&[
        item("A", &[], "tasks/A.md"),
        item("B", &["A", "C"], "tasks/B.md"),
        item("C", &["B"], "tasks/C.md"),
    ]);

    let problem = problem(manifest(&index).file("tasks/C.md", "# C\n")).await;

    assert_eq!(
        problem,
        ItemListProblem::Cycle {
            path: vec!["B".to_owned(), "C".to_owned(), "B".to_owned()]
        }
    );
}

#[tokio::test]
async fn an_item_that_needs_itself_is_a_cycle() {
    let index = list(&[item("A", &["A"], "tasks/A.md")]);

    let problem = problem(manifest(&index)).await;

    assert_eq!(
        problem,
        ItemListProblem::Cycle {
            path: vec!["A".to_owned(), "A".to_owned()]
        }
    );
}

#[tokio::test]
async fn the_index_is_checked_against_its_schema_first() {
    let index = r#"[{ "key": "A", "needs": [] }]"#;

    let rejection = file_rejection(manifest(index)).await;

    assert!(
        matches!(&rejection, DeliveryRejection::SchemaViolation { path, .. } if path == "manifest.json"),
        "{rejection:?}"
    );
}
