use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::json;
use surt_engine::project::{Error, Refusal};

use super::{Failure, Workspace};

#[test]
fn a_folder_without_surt_reads_as_a_hint_to_run_surt_init_there() {
    let root = PathBuf::from("projects").join("shop");

    let failure = Failure::from(Error::Refused(Refusal::NotAProject { root: root.clone() }));

    assert_eq!(
        serde_json::to_value(&failure).expect("serializes"),
        json!(format!(
            "{} is not a Surt project: run surt init there first",
            root.display()
        ))
    );
}

#[tokio::test]
async fn reading_a_project_before_one_is_chosen_is_refused() {
    let workspace = Workspace {
        root: Mutex::new(None),
    };

    let refused = workspace.project().await.err();

    assert!(!workspace.has_project());
    assert_eq!(
        refused.map(|failure| failure.to_string()),
        Some("no Surt project is open".to_owned())
    );
}
