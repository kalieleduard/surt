use surt_engine::project::{Change, FileChange, RestoreImpact};

use super::{question, restored, unchanged};
use crate::message::Mark;
use crate::symbols::Symbols;

const SUBJECT: &str = "20261017-add-login";

fn impact(changes: Vec<FileChange>) -> RestoreImpact {
    RestoreImpact {
        artifact: "manifest".to_owned(),
        revision: 1,
        changes,
    }
}

fn change(path: &str, change: Change) -> FileChange {
    FileChange {
        path: path.to_owned(),
        change,
    }
}

#[test]
fn the_question_lists_each_file_that_is_lost_with_its_line_counts() {
    let changes = vec![
        change(
            "tasks/B.md",
            Change::Changed {
                added: 4,
                removed: 1,
            },
        ),
        change("tasks/C.md", Change::New),
        change("tasks/manifest.json", Change::Missing),
    ];

    assert_eq!(
        question(SUBJECT, &impact(changes)),
        "discard your unrecorded edit to the manifest of 20261017-add-login?\n\
         \x20 tasks/B.md           +4 −1\n\
         \x20 tasks/C.md           new file\n\
         \x20 tasks/manifest.json  missing\n"
    );
}

#[test]
fn the_outcome_names_the_revision_the_working_copy_is_back_to() {
    let restored = restored(&impact(vec![change("tasks/C.md", Change::New)]));
    let unchanged = unchanged(&impact(Vec::new()));

    assert_eq!(
        restored.plain(Mark::Success),
        "✔ restored manifest rev 1; your edit was discarded\n"
    );
    assert_eq!(
        unchanged.plain(Mark::Success),
        "✔ manifest already matches rev 1; nothing to restore\n"
    );
}

#[test]
fn the_question_and_the_outcome_print_only_ascii_in_ascii_mode() {
    let changes = vec![change(
        "tasks/B.md",
        Change::Changed {
            added: 4,
            removed: 1,
        },
    )];
    let lost = impact(changes);

    assert!(
        Symbols::Ascii
            .render(&question(SUBJECT, &lost))
            .ends_with("tasks/B.md  +4 -1\n")
    );
    assert_eq!(
        Symbols::Ascii.render(&restored(&lost).plain(Mark::Success)),
        "[ok] restored manifest rev 1; your edit was discarded\n"
    );
}
