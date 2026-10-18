use std::fs;

use tempfile::TempDir;

use super::planner::{TWO_TASKS, file_rejection, item, list, manifest, planner_contract, problem};
use crate::artifacts::{DeliveryRejection, ItemListProblem};
use crate::runtime::Delivery;

#[tokio::test]
async fn every_item_carries_every_declared_item_file() {
    let index = r#"[{ "key": "A", "needs": [], "title": "t" }]"#;

    let problem = problem(manifest(index)).await;

    assert_eq!(
        problem,
        ItemListProblem::MissingItemFile {
            key: "A".to_owned(),
            name: "task".to_owned()
        }
    );
}

#[tokio::test]
async fn an_item_file_name_the_contract_does_not_declare_is_rejected() {
    let index = r#"[{ "key": "A", "needs": [], "title": "t", "files": { "task": "tasks/A.md", "notes": "tasks/B.md" } }]"#;

    let problem = problem(manifest(index)).await;

    assert_eq!(
        problem,
        ItemListProblem::UndeclaredItemFile {
            key: "A".to_owned(),
            name: "notes".to_owned()
        }
    );
}

#[tokio::test]
async fn two_items_cannot_share_a_path_even_in_another_case() {
    let index = list(&[item("A", &[], "tasks/A.md"), item("B", &[], "tasks/a.md")]);

    let problem = problem(manifest(&index)).await;

    assert_eq!(
        problem,
        ItemListProblem::DuplicatePath {
            path: "tasks/a.md".to_owned()
        }
    );
}

#[tokio::test]
async fn item_files_follow_the_same_file_rules_as_outputs() {
    let missing = list(&[item("A", &[], "tasks/Z.md")]);
    let unsafe_path = list(&[item("A", &[], "../A.md")]);
    let empty = Delivery::new("done")
        .output(
            "manifest",
            "manifest.json",
            list(&[item("A", &[], "tasks/A.md")]),
        )
        .file("tasks/A.md", "");

    assert_eq!(
        file_rejection(manifest(&missing)).await,
        DeliveryRejection::MissingFile {
            path: "tasks/Z.md".to_owned()
        }
    );
    assert_eq!(
        file_rejection(manifest(&unsafe_path)).await,
        DeliveryRejection::UnsafePath {
            path: "../A.md".to_owned()
        }
    );
    assert_eq!(
        file_rejection(empty).await,
        DeliveryRejection::EmptyFile {
            path: "tasks/A.md".to_owned()
        }
    );
}

#[test]
fn a_human_edit_of_an_item_list_is_validated_with_the_same_rules() {
    let working_copy = TempDir::new().expect("a working copy");
    fs::create_dir(working_copy.path().join("tasks")).expect("the tasks folder");
    fs::write(working_copy.path().join("tasks/A.md"), "# A\n").expect("task A");
    fs::write(working_copy.path().join("tasks/B.md"), "# B\n").expect("task B");
    let rule = planner_contract()
        .into_output("manifest")
        .expect("the manifest output");
    let cyclic = list(&[
        item("A", &["B"], "tasks/A.md"),
        item("B", &["A"], "tasks/B.md"),
    ]);

    fs::write(working_copy.path().join("manifest.json"), TWO_TASKS).expect("the index");
    let valid = rule.validate(working_copy.path(), "manifest.json");
    fs::write(working_copy.path().join("manifest.json"), cyclic).expect("the index");
    let invalid = rule.validate(working_copy.path(), "manifest.json");

    assert_eq!(valid.expect("a valid edit").files.len(), 3);
    assert_eq!(
        invalid.expect_err("a cyclic edit"),
        DeliveryRejection::InvalidItemList {
            path: "manifest.json".to_owned(),
            problem: ItemListProblem::Cycle {
                path: vec!["A".to_owned(), "B".to_owned(), "A".to_owned()]
            }
        }
    );
}
