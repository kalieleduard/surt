use std::path::PathBuf;

use surt_engine::project::{BlockReason, BlockedStep, InterruptedMerge, LastFailure, StepLabel};

use super::report;
use crate::message::Mark;
use crate::symbols::BLOCKED;

const FROM: &str = "a11c9d0ffffffffffffffffffffffffffffffff0";
const TO: &str = "4d2e9a1ffffffffffffffffffffffffffffffff0";
const RESUME: &str = "surt start 20261017-add-login";

fn blocked_merge(code: &str) -> BlockedStep {
    BlockedStep {
        step: StepLabel {
            key: "merge".to_owned(),
            item: None,
            iteration: None,
        },
        reason: BlockReason::NotRetryable,
        last_failure: Some(LastFailure {
            execution: "exec-1".to_owned(),
            attempt: 2,
            code: code.to_owned(),
            message: "merge into main was interrupted".to_owned(),
            resets_at: None,
        }),
    }
}

fn interrupted(worktree: &str) -> InterruptedMerge {
    InterruptedMerge {
        branch: "main".to_owned(),
        worktree: PathBuf::from(worktree),
        from: FROM.to_owned(),
        to: TO.to_owned(),
    }
}

#[test]
fn an_interrupted_fast_forward_says_how_to_finish_or_undo_it_and_then_resume() {
    let merge = interrupted("/Users/you/surt");

    let printed = report(
        &[blocked_merge("merge_into_base_interrupted")],
        Some(&merge),
        RESUME.to_owned(),
    )
    .plain(Mark::Blocked);

    assert_eq!(
        printed,
        format!(
            "{BLOCKED} merge into main was interrupted; /Users/you/surt may be partially updated\n\
             \x20 finish it:  git -C /Users/you/surt merge --ff-only 4d2e9a1\n\
             \x20 or undo it: git -C /Users/you/surt reset --hard a11c9d0   (discards changes)\n\
             \x20 then: surt start 20261017-add-login\n"
        )
    );
}

#[test]
fn a_worktree_path_with_a_space_is_quoted_in_the_commands() {
    let merge = interrupted("/Users/you/my surt");

    let printed = report(
        &[blocked_merge("merge_into_base_interrupted")],
        Some(&merge),
        RESUME.to_owned(),
    )
    .plain(Mark::Blocked);

    assert!(
        printed.contains("finish it:  git -C \"/Users/you/my surt\" merge --ff-only 4d2e9a1\n"),
        "{printed}"
    );
}

#[test]
fn another_failure_of_the_step_keeps_the_usual_lines() {
    let merge = interrupted("/Users/you/surt");

    let printed = report(
        &[blocked_merge("git_failed")],
        Some(&merge),
        RESUME.to_owned(),
    )
    .plain(Mark::Blocked);

    assert!(
        printed.contains("merge blocked: cannot be retried\n"),
        "{printed}"
    );
    assert!(
        printed.ends_with(&format!("resume: {RESUME}\n")),
        "{printed}"
    );
}
