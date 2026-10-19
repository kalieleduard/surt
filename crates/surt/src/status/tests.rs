use std::path::PathBuf;

use surt_engine::project::{
    AgentSlots, BlockReason, BlockedStep, CleanupState, Commit, Conflict, Driver, GateStatus,
    GateView, IntegrationBranch, ItemListSummary, ItemState, ItemStatus, Kept, LaunchPause, Left,
    MergeIntoBase, PendingIntegration, RevisionOrigin, RevisionSummary, RunState, RunStatus,
    StepLabel, Unattended,
};

use serde_json::json;

mod items;
mod merge;

use super::{detail, json, overview};
use crate::driving::local_time;
use crate::symbols::Symbols;

const SUBJECT: &str = "20261017-add-login";
const BASE: &str = "a11c9d0e5f60718293a4b5c6d7e8f90112233445";
const OLD_TIP: &str = "9e04b7d0c1a2b3c4d5e6f708192a3b4c5d6e7f80";
const NEW_TIP: &str = "3c1aa20000000000000000000000000000000000";
const SHA256: &str = "5d41402abc4b2a76b9719d911017c592ae2fe96f0c2a4b5e2f0d9a1c3b4e5f60";

fn step(key: &str, item: Option<&str>, iteration: Option<u32>) -> StepLabel {
    StepLabel {
        key: key.to_owned(),
        item: item.map(str::to_owned),
        iteration,
    }
}

fn gate(step: &str, artifact: &str, status: GateStatus, edited: &[&str]) -> GateView {
    gate_over(step, artifact, status, edited, &[&format!("{artifact}.md")])
}

fn gate_over(
    step: &str,
    artifact: &str,
    status: GateStatus,
    edited: &[&str],
    files: &[&str],
) -> GateView {
    GateView {
        step: step.to_owned(),
        artifact: artifact.to_owned(),
        status,
        revision: match status {
            GateStatus::NotReached => None,
            _ => Some(RevisionSummary {
                number: 1,
                origin: RevisionOrigin::Human,
                sha256: SHA256.to_owned(),
                files: files.iter().map(|file| (*file).to_owned()).collect(),
                item_list: None,
            }),
        },
        decided_by: None,
        decided_at: None,
        working_copy: PathBuf::from(".surt/subjects").join(SUBJECT),
        edited: edited.iter().map(|file| (*file).to_owned()).collect(),
        missing: Vec::new(),
    }
}

fn listed(mut gate: GateView, items: usize, item_files: usize) -> GateView {
    if let Some(revision) = gate.revision.as_mut() {
        revision.item_list = Some(ItemListSummary { items, item_files });
    }
    gate
}

fn item(key: &str, state: ItemState) -> ItemStatus {
    ItemStatus {
        key: key.to_owned(),
        state,
        step: Some("build".to_owned()),
        iteration: Some(2),
        attempt: Some(1),
        since: None,
        merged_as: None,
        waits_for: Vec::new(),
        conflict: None,
    }
}

fn slots(in_use: u32) -> AgentSlots {
    AgentSlots { in_use, total: 3 }
}

fn stopped_with_everything_pending() -> RunStatus {
    RunStatus {
        subject: SUBJECT.to_owned(),
        run: 2,
        runs: 3,
        states: vec![RunState::AwaitingApproval, RunState::Blocked],
        ended_at: None,
        driver: None,
        base: Some(Commit {
            sha: BASE.to_owned(),
            branch: None,
        }),
        branch: None,
        merge_into_base: MergeIntoBase::Never,
        merge: None,
        slots: slots(3),
        paused: None,
        item_step: Some("implement".to_owned()),
        gates: vec![
            gate(
                "approve-spec",
                "spec",
                GateStatus::Approved { revision: 1 },
                &["spec.md"],
            ),
            gate("approve-tasks", "manifest", GateStatus::NotReached, &[]),
            gate(
                "approve-plan",
                "plan",
                GateStatus::Rejected { revision: 1 },
                &[],
            ),
        ],
        items: vec![
            item("A", ItemState::Queued),
            ItemStatus {
                conflict: Some(Conflict::Merge {
                    paths: vec!["src/auth.rs".to_owned(), "src/user.rs".to_owned()],
                }),
                ..item("B", ItemState::Interrupted)
            },
            item("C", ItemState::Cancelled),
            ItemStatus {
                waits_for: vec!["B".to_owned(), "C".to_owned()],
                ..item("D", ItemState::Pending)
            },
            ItemStatus {
                conflict: Some(Conflict::Checks {
                    failed: vec!["test".to_owned()],
                }),
                ..item("E", ItemState::Running)
            },
        ],
        blocked: vec![BlockedStep {
            step: step("check", Some("B"), Some(2)),
            reason: BlockReason::IterationsExhausted,
            last_failure: None,
        }],
        unattended: vec![Unattended {
            step: step("build", Some("A"), Some(2)),
            pid: 50122,
        }],
        integrating: vec![PendingIntegration {
            step: step("integrate", Some("B"), None),
            branch: format!("surt/{SUBJECT}"),
            from: OLD_TIP.to_owned(),
            to: NEW_TIP.to_owned(),
        }],
        cleanup: None,
    }
}

fn cancelled_and_cleaned() -> RunStatus {
    RunStatus {
        subject: SUBJECT.to_owned(),
        run: 1,
        runs: 1,
        states: vec![RunState::Cancelled],
        ended_at: Some(1_760_713_320),
        driver: None,
        base: None,
        branch: Some(IntegrationBranch {
            name: format!("surt/{SUBJECT}"),
            tip: NEW_TIP.to_owned(),
        }),
        merge_into_base: MergeIntoBase::Never,
        merge: None,
        slots: slots(0),
        paused: None,
        gates: vec![listed(
            gate_over(
                "approve-tasks",
                "manifest",
                GateStatus::Approved { revision: 1 },
                &[],
                &["manifest.json", "tasks/A.md", "tasks/B.md"],
            ),
            2,
            2,
        )],
        item_step: None,
        items: Vec::new(),
        blocked: Vec::new(),
        unattended: Vec::new(),
        integrating: Vec::new(),
        cleanup: Some(CleanupState::Finished {
            kept: vec![Kept::MovedBranch {
                branch: format!("surt/{SUBJECT}"),
            }],
        }),
    }
}

#[test]
fn the_detail_names_the_run_among_several_and_every_item_state_unattended_agent_and_pending_integration()
 {
    let text = detail::render(&stopped_with_everything_pending());

    assert_eq!(
        text,
        format!(
            "{SUBJECT} · run 2 of 3   AWAITING APPROVAL + BLOCKED   (no driver)\n  \
             base     HEAD @ a11c9d0\n  \
             spec     rev 1 (human) ✔ approved\n           \
             ✎ spec.md edited — not recorded\n  \
             plan     rev 1 (human) ✖ rejected\n  \
             implement\n    \
             A  ⏳ queued    build iter 2 · attempt 1 · waiting for agent slot (3/3 in use)\n    \
             B  ⏹ stopped   build iter 2 · attempt 1 · merge: src/auth.rs, src/user.rs\n    \
             C  ✖ cancelled\n    \
             D  ⏸ pending   waits for B, C\n    \
             E  ● build     iter 2 · attempt 1 · checks: test\n  \
             ⛔ item B check iter 2 blocked: iterations exhausted\n  \
             resume: surt continue {SUBJECT}\n  \
             ⚠ item A build iter 2 still running unattended (pid 50122) — stopped on next: surt continue {SUBJECT}\n  \
             ⟳ item B integrate   surt/{SUBJECT}: 9e04b7d → 3c1aa20 (pending)\n"
        )
    );
}

#[test]
fn a_cleaned_terminal_run_shows_what_its_cleanup_kept_and_no_driver_column() {
    let text = detail::render(&cancelled_and_cleaned());

    assert_eq!(
        text,
        format!(
            "{SUBJECT} · run 1   CANCELLED\n  \
             branch   surt/{SUBJECT} @ 3c1aa20 · merge into base: never\n  \
             manifest rev 1 (human) ✔ approved · 2 items\n  \
             cleanup  done\n           \
             kept surt/{SUBJECT} — moved outside surt\n"
        )
    );
}

#[test]
fn a_run_kept_by_its_cleanup_policy_says_how_to_remove_it() {
    let mut kept = cancelled_and_cleaned();
    kept.states = vec![RunState::Succeeded];
    kept.cleanup = Some(CleanupState::Kept);

    let detail = detail::render(&kept);
    let overview = overview::render(&[kept]);

    assert!(
        detail.ends_with(&format!(
            "  cleanup  kept (on_success = \"keep\") — remove with: surt delete {SUBJECT}\n"
        )),
        "{detail}"
    );
    assert!(
        overview.contains(&format!("{SUBJECT}  1    SUCCEEDED  —       kept\n")),
        "{overview}"
    );
}

#[test]
fn the_overview_joins_simultaneous_states_and_falls_back_to_the_blocked_step() {
    let mut blocked_at_spec = stopped_with_everything_pending();
    blocked_at_spec.items.clear();
    blocked_at_spec.blocked[0].step = step("spec", None, None);

    let text = overview::render(&[blocked_at_spec, cancelled_and_cleaned()]);

    assert_eq!(
        text,
        format!(
            "SUBJECT             RUN  STATE                        DRIVER  PROGRESS\n\
             {SUBJECT}  2    AWAITING APPROVAL + BLOCKED  —       spec blocked\n\
             {SUBJECT}  1    CANCELLED                    —       —\n\
             agent slots: 3/3 in use\n"
        )
    );
}

#[test]
fn the_overview_prints_the_project_wide_slot_count_once_not_once_per_subject() {
    let mut first = stopped_with_everything_pending();
    first.slots = slots(1);
    let mut second = cancelled_and_cleaned();
    second.slots = slots(1);

    let text = overview::render(&[first, second]);

    assert!(text.ends_with("agent slots: 1/3 in use\n"), "{text}");
}

#[test]
fn the_detail_in_ascii_mode_replaces_every_glyph() {
    let text = detail::render(&stopped_with_everything_pending());

    assert_eq!(
        Symbols::Ascii.render(&text),
        format!(
            "{SUBJECT} | run 2 of 3   AWAITING APPROVAL + BLOCKED   (no driver)\n  \
             base     HEAD @ a11c9d0\n  \
             spec     rev 1 (human) [ok] approved\n           \
             [edited] spec.md edited - not recorded\n  \
             plan     rev 1 (human) [error] rejected\n  \
             implement\n    \
             A  [waiting] queued    build iter 2 | attempt 1 | waiting for agent slot (3/3 in use)\n    \
             B  [stopped] stopped   build iter 2 | attempt 1 | merge: src/auth.rs, src/user.rs\n    \
             C  [error] cancelled\n    \
             D  [paused] pending   waits for B, C\n    \
             E  * build     iter 2 | attempt 1 | checks: test\n  \
             [blocked] item B check iter 2 blocked: iterations exhausted\n  \
             resume: surt continue {SUBJECT}\n  \
             [warn] item A build iter 2 still running unattended (pid 50122) - stopped on next: surt continue {SUBJECT}\n  \
             [integrating] item B integrate   surt/{SUBJECT}: 9e04b7d -> 3c1aa20 (pending)\n"
        )
    );
}

#[test]
fn the_overview_and_a_cleaned_detail_print_only_ascii_in_ascii_mode() {
    let texts = [
        overview::render(&[stopped_with_everything_pending(), cancelled_and_cleaned()]),
        detail::render(&cancelled_and_cleaned()),
    ];

    for text in texts {
        assert!(Symbols::Ascii.render(&text).is_ascii(), "{text}");
    }
}

#[test]
fn status_json_carries_the_cleanup_state_the_detail_shows() {
    let cleanup = |state| {
        let mut status = cancelled_and_cleaned();
        status.cleanup = state;
        serde_json::Value::from(json::subject(&status))["cleanup"].clone()
    };
    let moved = Kept::MovedBranch {
        branch: format!("surt/{SUBJECT}"),
    };
    let edited = Kept::UnrecordedEdit {
        path: PathBuf::from(".surt/working/x/spec.md"),
    };
    let left = Left {
        item: "worktree ~/w/B".to_owned(),
        reason: "locked".to_owned(),
    };

    assert_eq!(
        cleanup(Some(CleanupState::Finished {
            kept: vec![moved.clone()]
        })),
        json!({"state": "done", "kept": [{"kind": "moved_branch", "branch": format!("surt/{SUBJECT}")}]})
    );
    assert_eq!(
        cleanup(Some(CleanupState::Pending {
            left: vec![left],
            kept: vec![edited]
        })),
        json!({
            "state": "pending",
            "left": [{"item": "worktree ~/w/B", "reason": "locked"}],
            "kept": [{"kind": "unrecorded_edit", "path": ".surt/working/x/spec.md"}]
        })
    );
    assert_eq!(cleanup(Some(CleanupState::Kept)), json!({"state": "kept"}));
    assert_eq!(cleanup(None), serde_json::Value::Null);
}

const RESET: i64 = 1_792_900_800;

fn paused_with_an_item_waiting(resets_at: Option<i64>) -> RunStatus {
    RunStatus {
        states: vec![RunState::Running],
        driver: Some(Driver {
            pid: 48213,
            since: None,
        }),
        slots: slots(1),
        paused: Some(LaunchPause {
            code: "rate_limit".to_owned(),
            resets_at,
        }),
        gates: Vec::new(),
        items: vec![
            ItemStatus {
                iteration: Some(1),
                ..item("A", ItemState::Queued)
            },
            ItemStatus {
                iteration: Some(1),
                ..item("B", ItemState::Running)
            },
        ],
        blocked: Vec::new(),
        unattended: Vec::new(),
        integrating: Vec::new(),
        ..stopped_with_everything_pending()
    }
}

#[test]
fn a_paused_run_stays_running_and_names_the_error_code_and_the_reset_time() {
    let at = local_time(RESET).expect("a local time");

    let text = detail::render(&paused_with_an_item_waiting(Some(RESET)));

    assert_eq!(
        text,
        format!(
            "{SUBJECT} · run 2 of 3   RUNNING   driver pid 48213\n  \
             ⏸ agent launches paused: rate_limit — resets {at}\n  \
             base     HEAD @ a11c9d0\n  \
             implement\n    \
             A  ⏳ queued    build iter 1 · attempt 1 · waiting for agent launches to resume\n    \
             B  ● build     iter 1 · attempt 1\n"
        )
    );
}

#[test]
fn a_pause_without_a_reset_time_retries_with_backoff_and_reads_in_ascii() {
    let text = detail::render(&paused_with_an_item_waiting(None));

    let ascii = Symbols::Ascii.render(&text);

    assert!(
        ascii.contains("\n  [paused] agent launches paused: rate_limit - retrying with backoff\n"),
        "{ascii}"
    );
    assert!(ascii.is_ascii(), "{ascii}");
}

#[test]
fn status_json_carries_the_pause_with_its_reset_time_when_known() {
    let paused = |resets_at| {
        serde_json::Value::from(json::subject(&paused_with_an_item_waiting(resets_at)))["paused"]
            .clone()
    };

    assert_eq!(
        paused(Some(RESET)),
        json!({"code": "rate_limit", "resets_at": "2026-10-25T04:00:00Z"})
    );
    assert_eq!(paused(None), json!({"code": "rate_limit"}));
    let running = serde_json::Value::from(json::subject(&cancelled_and_cleaned()));
    assert_eq!(running.get("paused"), None);
}
