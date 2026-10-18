use surt_engine::project::{BaseMerge, BaseMergeReport, Commit, NotMerged};

use super::line;

const INTEGRATION: &str = "surt/20261017-add-login";
const BASE: &str = "a11c9d0ffffffffffffffffffffffffffffffff0";

fn report(branch: Option<&str>, merge: BaseMerge) -> BaseMergeReport {
    BaseMergeReport {
        base: Commit {
            sha: BASE.to_owned(),
            branch: branch.map(str::to_owned),
        },
        merge,
    }
}

fn printed(report: &BaseMergeReport) -> Option<String> {
    line(report, INTEGRATION).map(|(mark, message)| message.plain(mark))
}

#[test]
fn a_disabled_merge_prints_nothing() {
    let disabled = report(Some("main"), BaseMerge::Skipped(NotMerged::Disabled));

    assert_eq!(printed(&disabled), None);
}

#[test]
fn every_skip_names_its_reason_and_leaves_the_merge_to_the_developer() {
    let cases = [
        (
            report(None, BaseMerge::Skipped(NotMerged::NoBaseBranch)),
            "⚠ not merged: the base a11c9d0 is not a branch; merge surt/20261017-add-login yourself\n",
        ),
        (
            report(
                Some("main"),
                BaseMerge::Skipped(NotMerged::WorktreeNotClean),
            ),
            "⚠ not merged: main is checked out in a worktree that is not clean; merge surt/20261017-add-login yourself\n",
        ),
        (
            report(
                Some("main"),
                BaseMerge::Skipped(NotMerged::BaseMoved { found: None }),
            ),
            "⚠ not merged: main no longer exists (base a11c9d0); merge surt/20261017-add-login yourself\n",
        ),
    ];

    for (report, expected) in cases {
        assert_eq!(printed(&report).as_deref(), Some(expected));
    }
}
