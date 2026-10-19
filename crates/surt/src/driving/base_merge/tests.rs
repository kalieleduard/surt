use surt_engine::project::{BaseMerge, BaseMergeReport, Commit, NotMerged};

use super::line;
use crate::symbols::Symbols;

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

#[test]
fn every_merge_line_renders_as_ascii_without_a_utf8_console() {
    let merged = report(
        Some("main"),
        BaseMerge::Merged {
            from: BASE.to_owned(),
            to: "4e1b2c0ffffffffffffffffffffffffffffffff0".to_owned(),
        },
    );
    let moved = report(
        Some("main"),
        BaseMerge::Skipped(NotMerged::BaseMoved {
            found: Some("b22e310ffffffffffffffffffffffffffffffff0".to_owned()),
        }),
    );

    let rendered: Vec<String> = [merged, moved]
        .iter()
        .filter_map(printed)
        .map(|text| Symbols::Ascii.render(&text).into_owned())
        .collect();

    assert_eq!(
        rendered,
        [
            "[ok] merged into main (fast-forward a11c9d0 -> 4e1b2c0)\n",
            "[warn] not merged: main moved since base (a11c9d0 -> b22e310); merge surt/20261017-add-login yourself\n",
        ]
    );
}
