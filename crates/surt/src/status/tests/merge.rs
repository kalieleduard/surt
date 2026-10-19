use serde_json::{Value, json};
use surt_engine::project::{BaseMerge, Commit, MergeIntoBase, NotMerged, RunState, RunStatus};

use super::super::{detail, json};
use super::{BASE, NEW_TIP, OLD_TIP, cancelled_and_cleaned};

fn finished(branch: Option<&str>, merge: Option<BaseMerge>) -> RunStatus {
    RunStatus {
        states: vec![RunState::Succeeded],
        base: Some(Commit {
            sha: BASE.to_owned(),
            branch: branch.map(str::to_owned),
        }),
        merge_into_base: MergeIntoBase::FfOnly,
        merge,
        cleanup: None,
        ..cancelled_and_cleaned()
    }
}

fn merge_line(status: &RunStatus) -> Option<String> {
    detail::render(status)
        .lines()
        .find(|line| line.starts_with("  merge "))
        .map(str::to_owned)
}

fn merge_json(status: &RunStatus) -> Value {
    Value::from(json::feature(status))["merge"].clone()
}

fn merged() -> BaseMerge {
    BaseMerge::Merged {
        from: BASE.to_owned(),
        to: NEW_TIP.to_owned(),
    }
}

#[test]
fn the_detail_ends_a_merged_run_with_the_fast_forward_it_made() {
    let text = detail::render(&finished(Some("main"), Some(merged())));

    assert!(
        text.ends_with("  merge    merged into main (fast-forward a11c9d0 → 3c1aa20)\n"),
        "{text}"
    );
    assert!(text.contains("merge into base: ff-only"), "{text}");
}

#[test]
fn the_detail_names_why_a_run_was_not_merged() {
    let cases = [
        (
            NotMerged::BaseMoved {
                found: Some(OLD_TIP.to_owned()),
            },
            "  merge    not merged: main moved since base (a11c9d0 → 9e04b7d)",
        ),
        (
            NotMerged::BaseMoved { found: None },
            "  merge    not merged: main no longer exists (base a11c9d0)",
        ),
        (
            NotMerged::NoBaseBranch,
            "  merge    not merged: the base a11c9d0 is not a branch",
        ),
        (
            NotMerged::WorktreeNotClean {
                worktree: "/Users/you/surt".into(),
            },
            "  merge    not merged: main is checked out in /Users/you/surt with changes",
        ),
    ];

    for (reason, expected) in cases {
        let status = finished(Some("main"), Some(BaseMerge::Skipped(reason)));

        assert_eq!(merge_line(&status).as_deref(), Some(expected));
    }
}

#[test]
fn the_detail_has_no_merge_line_when_merging_was_disabled_or_has_not_ended() {
    let disabled = finished(Some("main"), Some(BaseMerge::Skipped(NotMerged::Disabled)));
    let unfinished = finished(Some("main"), None);

    assert_eq!(merge_line(&disabled), None);
    assert_eq!(merge_line(&unfinished), None);
}

#[test]
fn status_json_carries_the_merge_result_as_recorded() {
    let skipped = |reason| merge_json(&finished(Some("main"), Some(BaseMerge::Skipped(reason))));

    assert_eq!(
        merge_json(&finished(Some("main"), Some(merged()))),
        json!({"outcome": "merged", "from": BASE, "to": NEW_TIP})
    );
    assert_eq!(
        skipped(NotMerged::BaseMoved {
            found: Some(OLD_TIP.to_owned())
        }),
        json!({"outcome": "skipped", "reason": "base_moved", "found": OLD_TIP})
    );
    assert_eq!(
        skipped(NotMerged::BaseMoved { found: None }),
        json!({"outcome": "skipped", "reason": "base_moved"})
    );
    assert_eq!(
        skipped(NotMerged::NoBaseBranch),
        json!({"outcome": "skipped", "reason": "no_base_branch"})
    );
    assert_eq!(
        skipped(NotMerged::WorktreeNotClean {
            worktree: "/Users/you/surt".into()
        }),
        json!({"outcome": "skipped", "reason": "worktree_not_clean", "worktree": "/Users/you/surt"})
    );
    assert_eq!(
        skipped(NotMerged::Disabled),
        json!({"outcome": "skipped", "reason": "disabled"})
    );
    assert_eq!(merge_json(&finished(Some("main"), None)), Value::Null);
}
