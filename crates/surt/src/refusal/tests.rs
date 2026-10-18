use std::path::PathBuf;

mod executions;
mod problems;

use surt_engine::project::{
    AwaitedRevision, Candidate, DefinitionFile, GitVersion, InvalidDefinition, InvalidOverride,
    Problem, Refusal, RunState,
};

use super::message;
use crate::symbols::Symbols;

const MAX_CONTEXT_LINES: usize = 2;
const MAX_WAYS_OUT: usize = 3;
const LABELS: [&str; 10] = [
    "retry:",
    "fix:",
    "next:",
    "resume:",
    "regenerate:",
    "undo:",
    "inspect:",
    "or:",
    "run:",
    "review it, then:",
];

fn text(refusal: &Refusal) -> String {
    message(refusal).plain_error()
}

fn every_refusal() -> Vec<Refusal> {
    let path = PathBuf::from("/work/app");
    vec![
        Refusal::NotAProject { root: path.clone() },
        Refusal::InsideManagedWorktree {
            path: path.clone(),
            project: Some(path.clone()),
        },
        Refusal::InsideManagedWorktree {
            path: path.clone(),
            project: None,
        },
        Refusal::NotAGitRepository { path: path.clone() },
        Refusal::BareRepository { path: path.clone() },
        Refusal::GitNotFound,
        Refusal::GitTooOld {
            found: GitVersion {
                major: 2,
                minor: 34,
                patch: 1,
            },
            min: GitVersion {
                major: 2,
                minor: 38,
                patch: 0,
            },
        },
        Refusal::NoGitIdentity,
        Refusal::SchemaTooNew {
            db: 7,
            supported: 5,
        },
        Refusal::InvalidConfig {
            path: path.join(".surt/config.toml"),
            line: Some(2),
            reason: "unknown field `max_atempts`".to_owned(),
        },
        Refusal::InvalidConfig {
            path: path.join(".surt/config.toml"),
            line: None,
            reason: "invalid TOML".to_owned(),
        },
        Refusal::MergeIntoBaseUnsupported {
            path: path.join(".surt/config.toml"),
        },
        Refusal::InvalidDefinition(Box::new(InvalidDefinition {
            file: DefinitionFile::Workflow("feature".to_owned()),
            step: Some("ship".to_owned()),
            problem: Problem::MissingKind,
        })),
        Refusal::InvalidDefinition(Box::new(InvalidDefinition {
            file: DefinitionFile::Agent("builder".to_owned()),
            step: None,
            problem: Problem::MissingClaudeProfile,
        })),
        Refusal::DriverLive {
            subject: "20261017-add-login".to_owned(),
            pid: Some(48213),
            since: Some(1_792_000_000),
        },
        Refusal::DriverLive {
            subject: "20261017-add-login".to_owned(),
            pid: None,
            since: None,
        },
        Refusal::NoGateWaiting {
            subject: "20261017-add-login".to_owned(),
        },
        Refusal::StaleRevision {
            subject: "20261017-add-login".to_owned(),
            artifact: "spec".to_owned(),
            expected: 1,
            latest: 2,
        },
        Refusal::UnknownSubject {
            reference: "login".to_owned(),
        },
        invalid_override(Some("profile.effort")),
        Refusal::UnapprovedRevision {
            subject: "20261017-add-login".to_owned(),
            artifact: "spec".to_owned(),
            revision: 2,
        },
        Refusal::PinnedEdit {
            subject: "20261017-add-login".to_owned(),
            artifact: "manifest".to_owned(),
            revision: 1,
            files: vec!["tasks/B.md".to_owned(), "tasks/C.md".to_owned()],
        },
        Refusal::InvalidEdit {
            subject: "20261017-add-login".to_owned(),
            artifact: "manifest".to_owned(),
            problem: "manifest.json: tasks/C.md is not referenced by any item".to_owned(),
            files: vec!["tasks/C.md".to_owned()],
        },
        Refusal::IntegrationBranchMoved {
            subject: "20261017-add-login".to_owned(),
            branch: "surt/20261017-add-login".to_owned(),
            expected: "9e04b7d0c1a2b3c4d5e6f708192a3b4c5d6e7f80".to_owned(),
            found: Some("71aa0c2f00000000000000000000000000000000".to_owned()),
        },
        Refusal::IntegrationBranchMoved {
            subject: "20261017-add-login".to_owned(),
            branch: "surt/20261017-add-login".to_owned(),
            expected: "9e04b7d0c1a2b3c4d5e6f708192a3b4c5d6e7f80".to_owned(),
            found: None,
        },
        Refusal::IntegrationBranchCheckedOut {
            subject: "20261017-add-login".to_owned(),
            branch: "surt/20261017-add-login".to_owned(),
            path: path.clone(),
        },
        Refusal::UnknownArtifact {
            subject: "20261017-add-login".to_owned(),
            artifact: "specs".to_owned(),
            known: vec!["request".to_owned(), "spec".to_owned()],
        },
        Refusal::UnknownArtifact {
            subject: "20261017-add-login".to_owned(),
            artifact: "spec".to_owned(),
            known: Vec::new(),
        },
        Refusal::UnknownRevision {
            subject: "20261017-add-login".to_owned(),
            artifact: "spec".to_owned(),
            revision: 3,
            latest: 2,
        },
        Refusal::UnknownItem {
            subject: "20261017-add-login".to_owned(),
            item: "Z".to_owned(),
            known: vec!["A".to_owned(), "B".to_owned()],
        },
        Refusal::UnknownItem {
            subject: "20261017-add-login".to_owned(),
            item: "A".to_owned(),
            known: Vec::new(),
        },
        not_restorable("request", &["manifest", "spec"]),
        not_restorable("spec", &[]),
        terminal_run(),
        Refusal::NoLiveDriver {
            subject: "20261017-add-login".to_owned(),
            states: vec![RunState::AwaitingApproval, RunState::Blocked],
        },
        Refusal::ActiveRunBlocksDelete {
            subject: "20261017-add-login".to_owned(),
        },
        Refusal::RunLockBusy {
            subject: "20261017-add-login".to_owned(),
        },
        Refusal::SubjectChangedWhileDeleting {
            subject: "20261017-add-login".to_owned(),
        },
        Refusal::UnrecordedEditsBlockNewRun {
            subject: "20261017-add-login".to_owned(),
            run: 1,
            files: vec!["spec.md".to_owned(), "notes.md".to_owned()],
        },
        Refusal::ActiveRunExists {
            subject: "20261017-add-login".to_owned(),
        },
    ]
}

fn invalid_override(field: Option<&str>) -> Refusal {
    Refusal::InvalidAgentOverride(Box::new(InvalidOverride {
        file: ".surt/agents/builder/agent.toml".to_owned(),
        field: field.map(str::to_owned),
        problem: Problem::Syntax {
            line: None,
            detail:
                "unknown variant `huge`, expected one of `low`, `medium`, `high`, `xhigh`, `max`"
                    .to_owned(),
        },
    }))
}

#[test]
fn an_invalid_agent_override_names_the_file_the_field_and_the_reason() {
    let texts = [
        text(&invalid_override(Some("profile.effort"))),
        text(&invalid_override(None)),
    ];

    assert_eq!(
        texts,
        [
            "✖ .surt/agents/builder/agent.toml: profile.effort: unknown variant `huge`, expected one of `low`, `medium`, `high`, `xhigh`, `max`\n  \
             fix: edit .surt/agents/builder/agent.toml\n",
            "✖ .surt/agents/builder/agent.toml: unknown variant `huge`, expected one of `low`, `medium`, `high`, `xhigh`, `max`\n  \
             fix: edit .surt/agents/builder/agent.toml\n",
        ]
    );
}

fn not_restorable(artifact: &str, restorable: &[&str]) -> Refusal {
    Refusal::NotRestorable {
        subject: "20261017-add-login".to_owned(),
        artifact: artifact.to_owned(),
        restorable: restorable.iter().map(|name| (*name).to_owned()).collect(),
    }
}

fn terminal_run() -> Refusal {
    Refusal::TerminalRun {
        subject: "20261017-add-login".to_owned(),
        run: 1,
        files: vec!["spec.md".to_owned()],
    }
}

#[test]
fn an_artifact_without_a_working_copy_lists_the_restorable_ones() {
    let texts = [
        text(&not_restorable("request", &["manifest", "spec"])),
        text(&not_restorable("spec", &[])),
    ];

    assert_eq!(
        texts,
        [
            "✖ request has no working copy to restore\n  restorable: manifest, spec\n  inspect: surt status 20261017-add-login\n",
            "✖ spec has no working copy to restore\n  20261017-add-login has no artifact with a working copy\n  inspect: surt status 20261017-add-login\n",
        ]
    );
}

#[test]
fn a_terminal_run_keeps_the_edited_file_and_says_to_move_or_delete_it() {
    assert_eq!(
        text(&terminal_run()),
        "✖ 20261017-add-login run 1 has ended; restore only rewrites an active run\n    .surt/features/20261017-add-login/spec.md\n  fix: move or delete the file\n"
    );
}

#[test]
fn reading_refusals_list_what_exists_and_how_to_look_it_up() {
    let reading = named(|refusal| {
        matches!(
            refusal,
            Refusal::UnknownArtifact { .. }
                | Refusal::UnknownRevision { .. }
                | Refusal::UnknownItem { .. }
        )
    });

    assert_eq!(
        reading,
        [
            "✖ 20261017-add-login has no artifact \"specs\"\n  \
             artifacts: request, spec\n  \
             inspect: surt show 20261017-add-login\n",
            "✖ 20261017-add-login has no artifact \"spec\"\n  \
             it has no artifacts yet\n  \
             inspect: surt show 20261017-add-login\n",
            "✖ spec has no rev 3; the latest is rev 2\n  \
             inspect: surt show 20261017-add-login spec --rev 2\n",
            "✖ 20261017-add-login has no task \"Z\"\n  \
             tasks: A, B\n  \
             inspect: surt status 20261017-add-login\n",
            "✖ 20261017-add-login has no task \"A\"\n  \
             it has no tasks yet\n  \
             inspect: surt status 20261017-add-login\n",
        ]
    );
}

fn named(predicate: fn(&Refusal) -> bool) -> Vec<String> {
    every_refusal()
        .iter()
        .filter(|refusal| predicate(refusal))
        .map(text)
        .collect()
}

#[test]
fn a_moved_integration_branch_names_both_commits_and_the_exact_command_to_restore_it() {
    let moved = named(|refusal| matches!(refusal, Refusal::IntegrationBranchMoved { .. }));

    assert_eq!(
        moved,
        [
            "✖ surt/20261017-add-login moved outside surt (expected 9e04b7d, found 71aa0c2)\n  \
             fix: git branch -f surt/20261017-add-login 9e04b7d0c1a2b3c4d5e6f708192a3b4c5d6e7f80\n  \
             resume: surt start 20261017-add-login\n",
            "✖ surt/20261017-add-login moved outside surt (expected 9e04b7d, found no branch)\n  \
             fix: git branch -f surt/20261017-add-login 9e04b7d0c1a2b3c4d5e6f708192a3b4c5d6e7f80\n  \
             resume: surt start 20261017-add-login\n",
        ]
    );
}

#[test]
fn a_checked_out_integration_branch_says_where_and_how_to_switch_away() {
    let checked_out =
        named(|refusal| matches!(refusal, Refusal::IntegrationBranchCheckedOut { .. }));

    assert_eq!(
        checked_out,
        ["✖ surt/20261017-add-login is checked out in /work/app\n  \
          fix: git -C /work/app switch --detach\n  \
          resume: surt start 20261017-add-login\n"]
    );
}

#[test]
fn every_refusal_has_one_headline_at_most_two_context_lines_and_one_to_three_ways_out() {
    for refusal in every_refusal() {
        let text = text(&refusal);
        let mut lines = text.lines();
        let headline = lines.next().expect("a headline");
        let rest: Vec<&str> = lines.collect();
        let ways = rest
            .iter()
            .filter(|line| {
                LABELS
                    .iter()
                    .any(|label| line.trim_start().starts_with(label))
            })
            .count();

        assert!(headline.starts_with("✖ "), "{text}");
        assert!(rest.iter().all(|line| line.starts_with("  ")), "{text}");
        assert!((1..=MAX_WAYS_OUT).contains(&ways), "{text}");
        assert!(rest.len() - ways <= MAX_CONTEXT_LINES, "{text}");
    }
}

#[test]
fn an_old_git_names_the_found_and_the_required_version() {
    let refusal = &every_refusal()[6];

    assert!(
        text(refusal).starts_with("✖ git 2.34.1 is too old; surt needs git 2.38 or later\n"),
        "{}",
        text(refusal)
    );
}

#[test]
fn a_newer_database_names_both_schema_versions() {
    let refusal = Refusal::SchemaTooNew {
        db: 7,
        supported: 5,
    };

    assert!(
        text(&refusal).starts_with("✖ .surt/surt.db is at schema 7; this surt supports up to 5\n")
    );
}

#[test]
fn a_managed_worktree_points_back_at_the_project_when_it_is_known() {
    let refusal = Refusal::InsideManagedWorktree {
        path: PathBuf::from("/home/you/.surt/worktrees/app-a1b2c3/20261017-add-login/A"),
        project: Some(PathBuf::from("/work/app")),
    };

    assert!(
        text(&refusal).contains("\n  fix: cd /work/app\n"),
        "{}",
        text(&refusal)
    );
}

#[test]
fn an_invalid_definition_names_the_file_and_the_step() {
    let refusal = &every_refusal()[12];

    assert!(
        text(refusal).starts_with("✖ invalid built-in definition: workflow feature [steps.ship]\n"),
        "{}",
        text(refusal)
    );
}

#[test]
fn an_invalid_definition_says_in_words_what_is_wrong() {
    let refusal = Refusal::InvalidDefinition(Box::new(InvalidDefinition {
        file: DefinitionFile::Workflow("task-cycle".to_owned()),
        step: Some("cycle".to_owned()),
        problem: Problem::UnknownOutcome {
            step: "integrate".to_owned(),
            outcome: "merge".to_owned(),
            declared: vec!["merged".to_owned(), "conflict".to_owned()],
        },
    }));

    assert_eq!(
        text(&refusal),
        "✖ invalid built-in definition: workflow task-cycle [steps.cycle]\n  expects outcome \"merge\" of \"integrate\", which declares: merged, conflict\n  fix: reinstall surt\n"
    );
}

#[test]
fn a_live_driver_is_named_by_its_pid() {
    let refusal = &every_refusal()[14];

    assert_eq!(
        text(refusal),
        "✖ 20261017-add-login is being driven by pid 48213\n  fix: surt stop 20261017-add-login, or Ctrl-C in its terminal, then rerun\n"
    );
}

#[test]
fn a_stale_revision_names_the_latest_and_how_to_read_it() {
    let refusal = &every_refusal()[17];

    assert_eq!(
        text(refusal),
        "✖ spec rev 2 is the latest, not rev 1\n  review rev 2 before deciding\n  run: surt show 20261017-add-login spec --rev 2\n"
    );
}

#[test]
fn an_active_run_points_at_resuming_the_feature() {
    let refusal = every_refusal().pop().expect("a refusal");

    assert_eq!(
        text(&refusal),
        "✖ 20261017-add-login already has an active run\n  resume: surt feature 20261017-add-login\n"
    );
}

#[test]
fn unrecorded_edits_left_by_the_previous_run_are_listed_before_a_new_run() {
    let single = Refusal::UnrecordedEditsBlockNewRun {
        subject: "20261017-add-login".to_owned(),
        run: 1,
        files: vec!["spec.md".to_owned()],
    };
    let refusals = every_refusal();
    let several = &refusals[refusals.len() - 2];

    assert_eq!(
        text(&single),
        "✖ working copy has unrecorded edits from run 1\n    .surt/features/20261017-add-login/spec.md\n  fix: move or delete the file, then run again\n"
    );
    assert_eq!(
        text(several),
        "✖ working copy has unrecorded edits from run 1\n    .surt/features/20261017-add-login/spec.md .surt/features/20261017-add-login/notes.md\n  fix: move or delete the files, then run again\n"
    );
}

#[test]
fn an_unapproved_edit_points_at_approving_it() {
    let refusal = &every_refusal()[20];

    assert_eq!(
        text(refusal),
        "✖ spec changed after approval (rev 2 not approved)\n  review it, then: surt approve 20261017-add-login\n"
    );
}

#[test]
fn an_edit_to_a_pinned_revision_says_how_to_undo_it() {
    let refusal = &every_refusal()[21];

    assert_eq!(
        text(refusal),
        "✖ tasks/B.md, tasks/C.md changed, but manifest rev 1 is pinned (a later step already read it)\n  the edit was not recorded\n  undo: surt restore 20261017-add-login manifest\n"
    );
}

#[test]
fn an_invalid_edit_says_what_is_wrong_and_how_to_undo_it() {
    let refusal = &every_refusal()[22];

    assert_eq!(
        text(refusal),
        "✖ manifest edit is invalid, no revision recorded:\n    manifest.json: tasks/C.md is not referenced by any item\n  undo: surt restore 20261017-add-login manifest\n"
    );
}

#[test]
fn an_unknown_feature_points_at_the_list_of_features() {
    let refusal = &every_refusal()[18];

    assert_eq!(
        text(refusal),
        "✖ no feature matches \"login\"\n  inspect: surt status\n"
    );
}

#[test]
fn an_ambiguous_feature_lists_every_candidate_and_asks_for_the_full_name() {
    let refusal = Refusal::AmbiguousSubject {
        reference: "add-login".to_owned(),
        candidates: vec![
            Candidate {
                name: "20261015-add-login".to_owned(),
                states: vec![RunState::AwaitingApproval],
                awaiting: Some(AwaitedRevision {
                    artifact: "spec".to_owned(),
                    revision: 1,
                }),
            },
            Candidate {
                name: "20261017-add-login".to_owned(),
                states: vec![RunState::Running],
                awaiting: None,
            },
            Candidate {
                name: "20261017-add-login-2".to_owned(),
                states: vec![RunState::AwaitingApproval, RunState::Blocked],
                awaiting: Some(AwaitedRevision {
                    artifact: "manifest".to_owned(),
                    revision: 2,
                }),
            },
        ],
    };

    assert_eq!(
        text(&refusal),
        "✖ \"add-login\" matches 3 features:\n    \
         20261015-add-login    AWAITING APPROVAL (spec rev 1)\n    \
         20261017-add-login    RUNNING\n    \
         20261017-add-login-2  AWAITING APPROVAL (manifest rev 2) + BLOCKED\n  \
         use the full name\n"
    );
}

#[test]
fn every_refusal_prints_only_ascii_in_ascii_mode() {
    for refusal in every_refusal() {
        let text = text(&refusal);

        assert!(Symbols::Ascii.render(&text).is_ascii(), "{text}");
    }
}

#[test]
fn delete_refusals_point_at_cancel_or_at_running_the_delete_again() {
    let refusals = every_refusal();
    let texts: Vec<String> = refusals[refusals.len() - 5..refusals.len() - 2]
        .iter()
        .map(text)
        .collect();

    assert_eq!(
        texts,
        [
            "✖ 20261017-add-login has an active run; delete never stops agents\n  \
             fix: surt cancel 20261017-add-login\n",
            "✖ another surt process is working on 20261017-add-login\n  \
             retry: surt delete 20261017-add-login\n",
            "✖ feature changed while deleting; run again\n  \
             retry: surt delete 20261017-add-login\n",
        ]
    );
}
