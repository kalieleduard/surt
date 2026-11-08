use std::path::PathBuf;

use serde_json::json;
use surt_engine::project::{
    AgentSlots, BlockReason, BlockedStep, Driver, GateKind, GateStatus, GateView,
    IntegrationBranch, ItemState, ItemStatus, LastFailure, MergeIntoBase, RunState, RunStatus,
    StepLabel,
};

use super::SubjectView;

const SUBJECT: &str = "20261107-login";

fn waiting_gate() -> GateView {
    GateView {
        step: "approve-tasks".to_owned(),
        kind: GateKind::Approval,
        artifact: "tasks".to_owned(),
        round: None,
        status: GateStatus::Waiting { revision: 2 },
        revision: None,
        decided_by: None,
        decided_at: None,
        working_copy: PathBuf::from(".surt/subjects").join(SUBJECT),
        edited: Vec::new(),
        missing: Vec::new(),
    }
}

fn blocked_task() -> ItemStatus {
    ItemStatus {
        key: "B".to_owned(),
        state: ItemState::Blocked,
        step: Some("check".to_owned()),
        iteration: Some(2),
        attempt: Some(3),
        since: Some(1_800_000_000),
        merged_as: None,
        needs: vec!["A".to_owned()],
        waits_for: Vec::new(),
        conflict: None,
    }
}

fn exhausted_check() -> BlockedStep {
    BlockedStep {
        step: StepLabel {
            key: "check".to_owned(),
            item: Some("B".to_owned()),
            iteration: Some(2),
        },
        reason: BlockReason::AttemptsExhausted,
        last_failure: Some(LastFailure {
            execution: "e7".to_owned(),
            attempt: 3,
            code: "exit_1".to_owned(),
            message: "tests failed".to_owned(),
            resets_at: None,
        }),
    }
}

fn status() -> RunStatus {
    RunStatus {
        subject: SUBJECT.to_owned(),
        run: 2,
        runs: 2,
        states: vec![RunState::AwaitingApproval, RunState::Blocked],
        ended_at: None,
        driver: Some(Driver {
            pid: 4242,
            since: Some(1_800_000_000),
        }),
        base: None,
        branch: Some(IntegrationBranch {
            name: "surt/20261107-login.2".to_owned(),
            tip: "3c1aa2".to_owned(),
        }),
        merge_into_base: MergeIntoBase::Never,
        merge: None,
        slots: AgentSlots {
            in_use: 1,
            total: 3,
        },
        paused: None,
        gates: vec![waiting_gate()],
        item_step: Some("implement".to_owned()),
        items: vec![blocked_task()],
        blocked: vec![exhausted_check()],
        unattended: Vec::new(),
        integrating: Vec::new(),
        cleanup: None,
    }
}

#[test]
fn a_subject_reads_in_camel_case_with_snake_case_states() {
    let subject = serde_json::to_value(SubjectView::from(status())).expect("serializes");

    assert_eq!(
        subject,
        json!({
            "name": SUBJECT,
            "run": 2,
            "runs": 2,
            "states": ["awaiting_approval", "blocked"],
            "endedAt": null,
            "driven": true,
            "branch": "surt/20261107-login.2",
            "itemStep": "implement",
            "items": [{
                "key": "B", "state": "blocked", "step": "check", "iteration": 2, "attempt": 3,
                "since": 1_800_000_000, "mergedAs": null, "needs": ["A"], "waitsFor": []
            }],
            "gates": [{
                "step": "approve-tasks", "kind": "approval", "artifact": "tasks", "status": "waiting",
                "revision": 2, "decidedBy": null, "decidedAt": null
            }],
            "blocked": [{
                "step": "check", "item": "B", "iteration": 2, "reason": "attempts_exhausted",
                "failure": "tests failed"
            }]
        })
    );
}
