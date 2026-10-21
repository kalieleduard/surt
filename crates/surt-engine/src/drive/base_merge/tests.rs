use serde_json::json;

use super::{BaseMerge, NotMerged};

#[test]
fn a_skip_records_its_reason_beside_the_outcome() {
    let moved = BaseMerge::Skipped(NotMerged::BaseMoved {
        found: Some("b22e310".to_owned()),
    });

    assert_eq!(
        moved.data().expect("serialises"),
        json!({ "outcome": "skipped", "reason": "base_moved", "found": "b22e310" })
    );
    assert_eq!(
        BaseMerge::Skipped(NotMerged::Disabled)
            .data()
            .expect("serialises"),
        json!({ "outcome": "skipped", "reason": "disabled" })
    );
}

#[test]
fn recorded_data_reads_back_and_an_older_event_reads_as_nothing() {
    let data = r#"{"outcome": "merged", "from": "a11c9d0", "to": "4e1b2c0"}"#;

    assert_eq!(
        BaseMerge::from_data(data),
        Some(BaseMerge::Merged {
            from: "a11c9d0".to_owned(),
            to: "4e1b2c0".to_owned(),
            merge_commit: false
        })
    );
    assert_eq!(BaseMerge::from_data(r#"{"outcome": "done"}"#), None);
}

#[test]
fn the_outcome_and_reason_names_are_the_recorded_tags() {
    let merges = [
        BaseMerge::Merged {
            from: "a11c9d0".to_owned(),
            to: "4e1b2c0".to_owned(),
            merge_commit: false,
        },
        BaseMerge::Merged {
            from: "a11c9d0".to_owned(),
            to: "4e1b2c0".to_owned(),
            merge_commit: true,
        },
        BaseMerge::Skipped(NotMerged::Disabled),
        BaseMerge::Skipped(NotMerged::NoBaseBranch),
        BaseMerge::Skipped(NotMerged::BaseMoved { found: None }),
        BaseMerge::Skipped(NotMerged::WorktreeNotClean {
            worktree: "/work/surt".into(),
        }),
        BaseMerge::Skipped(NotMerged::Conflict {
            paths: vec!["a".to_owned()],
        }),
    ];

    for merge in merges {
        let data = merge.data().expect("serialises");
        assert_eq!(data["outcome"], merge.outcome());
        if let BaseMerge::Skipped(skipped) = &merge {
            assert_eq!(data["reason"], skipped.reason());
        }
    }
}

#[test]
fn a_worktree_skip_records_the_worktree_and_an_older_one_reads_without_it() {
    let skipped = BaseMerge::Skipped(NotMerged::WorktreeNotClean {
        worktree: "/work/surt".into(),
    });

    assert_eq!(
        skipped.data().expect("serialises"),
        json!({ "outcome": "skipped", "reason": "worktree_not_clean", "worktree": "/work/surt" })
    );
    assert_eq!(
        BaseMerge::from_data(r#"{"outcome": "skipped", "reason": "worktree_not_clean"}"#),
        Some(BaseMerge::Skipped(NotMerged::WorktreeNotClean {
            worktree: "".into()
        }))
    );
}

#[test]
fn a_fast_forward_records_no_merge_commit_marker() {
    let fast_forward = BaseMerge::Merged {
        from: "a11c9d0".to_owned(),
        to: "4e1b2c0".to_owned(),
        merge_commit: false,
    };

    assert_eq!(
        fast_forward.data().expect("serialises"),
        json!({ "outcome": "merged", "from": "a11c9d0", "to": "4e1b2c0" })
    );
}

#[test]
fn a_merge_commit_records_its_marker_and_reads_back() {
    let merged = BaseMerge::Merged {
        from: "a11c9d0".to_owned(),
        to: "4e1b2c0".to_owned(),
        merge_commit: true,
    };
    let data = json!({
        "outcome": "merged", "from": "a11c9d0", "to": "4e1b2c0", "merge_commit": true
    });

    assert_eq!(merged.data().expect("serialises"), data);
    assert_eq!(BaseMerge::from_data(&data.to_string()), Some(merged));
}

#[test]
fn a_conflict_records_its_paths_and_reads_back() {
    let conflict = BaseMerge::Skipped(NotMerged::Conflict {
        paths: vec!["a".to_owned(), "b".to_owned()],
    });
    let data = json!({ "outcome": "skipped", "reason": "conflict", "paths": ["a", "b"] });

    assert_eq!(conflict.data().expect("serialises"), data);
    assert_eq!(BaseMerge::from_data(&data.to_string()), Some(conflict));
    assert_eq!(
        NotMerged::Conflict { paths: Vec::new() }.reason(),
        "conflict"
    );
}
