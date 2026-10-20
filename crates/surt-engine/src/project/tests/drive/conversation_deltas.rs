use std::fs;

use super::conversations::{continuing, only, reject, rejected_after};
use super::fixtures::{Recording, draft};
use crate::runtime::Script;

const EDITED: &str = "# Draft 1, edited by hand\n";

#[tokio::test(flavor = "multi_thread")]
async fn the_delta_holds_only_what_changed_and_never_the_agents_own_output() {
    let feature = continuing().await;
    rejected_after(&feature, 1).await;
    let runtime = Recording::new(Script::new().on("write", 2, 1, draft("# Draft 2\n")));

    feature.drive_with(&runtime).await;

    let request = only(runtime.requests());
    let delta = request.continuation.expect("a continuation").context;
    let written = request
        .delivery_dir
        .parent()
        .expect("the Execution directory")
        .join("context-continued.md");
    assert!(
        delta.contains("- draft-rejection rev 1   (iteration 1, approve)"),
        "{delta}"
    );
    assert!(!delta.contains("- draft rev"), "{delta}");
    assert!(!delta.contains("- request rev"), "{delta}");
    assert!(
        delta.contains(
            "## Why this Iteration started\nIteration 1 ended: write = done, approve = rejected\n"
        ),
        "{delta}"
    );
    assert!(
        request.context.contains("- draft rev 1"),
        "{}",
        request.context
    );
    assert!(
        request.context.contains("- request rev 1"),
        "{}",
        request.context
    );
    let file = fs::read_to_string(written).expect("context-continued.md");
    assert!(file.contains("- draft-rejection rev 1"), "{file}");
    assert!(!file.contains("- draft rev"), "{file}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_human_edit_of_the_agents_output_is_part_of_the_delta() {
    let feature = continuing().await;
    feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await
        .outcome
        .expect("the draft reaches the gate");
    feature.edit("draft.md", EDITED);
    reject(&feature).await;
    let runtime = Recording::new(Script::new().on("write", 2, 1, draft("# Draft 2\n")));

    feature.drive_with(&runtime).await;

    let delta = only(runtime.requests())
        .continuation
        .expect("a continuation")
        .context;
    assert!(delta.contains("- draft rev 2   (edit)"), "{delta}");
    assert!(delta.contains("- draft-rejection rev 1"), "{delta}");
}
