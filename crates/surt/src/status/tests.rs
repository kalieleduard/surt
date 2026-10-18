use std::path::PathBuf;

use surt_engine::project::{
    AgentSlots, BlockReason, BlockedStep, CleanupState, Commit, Conflict, GateStatus, GateView,
    IntegrationBranch, ItemState, ItemStatus, Kept, MergeIntoBase, PendingIntegration,
    RevisionOrigin, RevisionSummary, RunState, RunStatus, StepLabel, Unattended,
};

use super::{detail, overview};

const SUBJECT: &str = "20261017-add-login";
const BASE: &str = "a11c9d0e5f60718293a4b5c6d7e8f90112233445";
const OLD_TIP: &str = "9e04b7d0c1a2b3c4d5e6f708192a3b4c5d6e7f80";
const NEW_TIP: &str = "3c1aa20000000000000000000000000000000000";

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
                files: files.iter().map(|file| (*file).to_owned()).collect(),
            }),
        },
        decided_by: None,
        decided_at: None,
        working_copy: PathBuf::from(".surt/features").join(SUBJECT),
        edited: edited.iter().map(|file| (*file).to_owned()).collect(),
        missing: Vec::new(),
    }
}

fn task(key: &str, state: ItemState) -> ItemStatus {
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
        states: vec![RunState::AwaitingApproval, RunState::Blocked],
        driver: None,
        base: Some(Commit {
            sha: BASE.to_owned(),
            branch: None,
        }),
        branch: None,
        merge_into_base: MergeIntoBase::Never,
        slots: slots(3),
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
            task("A", ItemState::Queued),
            ItemStatus {
                conflict: Some(Conflict::Merge {
                    paths: vec!["src/auth.rs".to_owned(), "src/user.rs".to_owned()],
                }),
                ..task("B", ItemState::Interrupted)
            },
            task("C", ItemState::Cancelled),
            ItemStatus {
                waits_for: vec!["B".to_owned(), "C".to_owned()],
                ..task("D", ItemState::Pending)
            },
            ItemStatus {
                conflict: Some(Conflict::Checks {
                    failed: vec!["test".to_owned()],
                }),
                ..task("E", ItemState::Running)
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
        states: vec![RunState::Cancelled],
        driver: None,
        base: None,
        branch: Some(IntegrationBranch {
            name: format!("surt/{SUBJECT}"),
            tip: NEW_TIP.to_owned(),
        }),
        merge_into_base: MergeIntoBase::Never,
        slots: slots(0),
        gates: vec![gate_over(
            "approve-tasks",
            "manifest",
            GateStatus::Approved { revision: 1 },
            &[],
            &["manifest.json", "tasks/A.md", "tasks/B.md"],
        )],
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
fn the_detail_names_every_task_state_unattended_agent_and_pending_integration() {
    let text = detail::render(&stopped_with_everything_pending());

    assert_eq!(
        text,
        format!(
            "{SUBJECT} · run 2   AWAITING APPROVAL + BLOCKED   (no driver)\n  \
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
             ⛔ task B check iter 2 blocked: iterations exhausted\n  \
             resume: surt plan {SUBJECT}\n  \
             ⚠ task A build iter 2 still running unattended (pid 50122) — stopped on next: surt plan {SUBJECT}\n  \
             ⟳ task B integrate   surt/{SUBJECT}: 9e04b7d → 3c1aa20 (pending)\n"
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
             tasks    manifest rev 1 (human) ✔ approved · 2 tasks\n  \
             cleanup  done\n           \
             kept surt/{SUBJECT} — moved outside surt\n"
        )
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
            "FEATURE             RUN  STATE                        DRIVER  PROGRESS\n\
             {SUBJECT}  2    AWAITING APPROVAL + BLOCKED  —       spec blocked\n\
             {SUBJECT}  1    CANCELLED                    —       —\n\
             agent slots: 3/3 in use\n"
        )
    );
}
