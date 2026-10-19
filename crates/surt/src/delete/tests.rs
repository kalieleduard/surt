use std::path::PathBuf;

use surt_engine::project::{BranchFate, DeleteImpact, DeletedRun, Directory, Left, RunState};

use super::incomplete;
use super::question::{question, size};
use crate::message::Mark;
use crate::symbols::Symbols;

const SUBJECT: &str = "20261017-add-login";

fn directory(path: &str, bytes: u64) -> Directory {
    Directory {
        path: PathBuf::from(path),
        bytes,
    }
}

fn impact() -> DeleteImpact {
    DeleteImpact {
        subject: SUBJECT.to_owned(),
        runs: vec![
            DeletedRun {
                run: 1,
                state: RunState::Cancelled,
            },
            DeletedRun {
                run: 2,
                state: RunState::Succeeded,
            },
        ],
        integration_branches: vec![
            BranchFate {
                branch: format!("surt/{SUBJECT}"),
                run: 1,
                kept: false,
            },
            BranchFate {
                branch: format!("surt/{SUBJECT}.2"),
                run: 2,
                kept: true,
            },
        ],
        item_branches: vec![
            format!("surt/task/{SUBJECT}/A"),
            format!("surt/task/{SUBJECT}/B"),
            format!("surt/task/{SUBJECT}.2/C"),
        ],
        salvage_refs: 1,
        directories: vec![
            directory(&format!(".surt/logs/{SUBJECT}"), 14 * 1024 * 1024),
            directory(&format!(".surt/features/{SUBJECT}"), 2048),
            directory(".surt/runtime/executions/0192", 10),
            directory(".surt/runtime/executions/0193", 20),
        ],
        unrecorded_edits: vec![PathBuf::from(format!(".surt/features/{SUBJECT}/spec.md"))],
        revisions: 3,
        executions: 41,
        events: 312,
    }
}

#[test]
fn the_question_lists_runs_refs_disk_and_history() {
    assert_eq!(
        question(&impact()),
        "delete subject 20261017-add-login and all its history?\n\
         \x20 runs      1 cancelled · 2 succeeded\n\
         \x20 git       delete surt/20261017-add-login (run 1, cancelled)\n\
         \x20           delete surt/task/20261017-add-login/{A,B} · surt/task/20261017-add-login.2/C · 1 salvage ref\n\
         \x20           keep   surt/20261017-add-login.2 (run 2, delivered)\n\
         \x20 disk      .surt/logs/20261017-add-login (14 MB) · .surt/features/20261017-add-login (2 KB) \
         · 2 execution directories (30 B) · 1 unrecorded edit in the working copy\n\
         \x20 database  3 revisions · 41 executions · 312 events\n"
    );
}

#[test]
fn the_question_says_when_git_and_disk_hold_nothing() {
    let mut empty = impact();
    empty.runs.truncate(1);
    empty.integration_branches.clear();
    empty.item_branches.clear();
    empty.salvage_refs = 2;
    empty.directories.clear();
    empty.unrecorded_edits.clear();
    empty.revisions = 1;
    empty.executions = 1;
    empty.events = 1;

    assert_eq!(
        question(&empty),
        "delete subject 20261017-add-login and all its history?\n\
         \x20 runs      1 cancelled\n\
         \x20 git       delete 2 salvage refs\n\
         \x20 disk      nothing\n\
         \x20 database  1 revision · 1 execution · 1 event\n"
    );
    empty.salvage_refs = 0;
    assert!(question(&empty).contains("  git       nothing\n"));
}

#[test]
fn sizes_use_the_largest_whole_unit() {
    for (bytes, shown) in [
        (0, "0 B"),
        (1023, "1023 B"),
        (1024, "1 KB"),
        (1536, "1 KB"),
        (5 * 1024 * 1024 * 1024, "5 GB"),
    ] {
        assert_eq!(size(bytes), shown);
    }
}

#[test]
fn an_incomplete_delete_names_what_is_left_and_how_to_retry() {
    let left = [
        Left {
            item: format!(".surt/logs/{SUBJECT}"),
            reason: "Permission denied".to_owned(),
        },
        Left {
            item: "refs".to_owned(),
            reason: "git failed".to_owned(),
        },
    ];

    assert_eq!(
        incomplete(SUBJECT, &left).plain(Mark::Error),
        "✖ delete incomplete — could not remove .surt/logs/20261017-add-login (Permission denied)\n\
         \x20 could not remove refs (git failed)\n\
         \x20 retry: surt delete 20261017-add-login\n"
    );
}

#[test]
fn the_question_and_an_incomplete_delete_print_only_ascii_in_ascii_mode() {
    let left = [Left {
        item: "refs".to_owned(),
        reason: "git failed".to_owned(),
    }];
    let texts = [
        question(&impact()),
        incomplete(SUBJECT, &left).plain(Mark::Error),
    ];

    for text in texts {
        assert!(Symbols::Ascii.render(&text).is_ascii(), "{text}");
    }
}
