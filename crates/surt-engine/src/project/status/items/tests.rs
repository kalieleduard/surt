use super::{Readiness, standing};
use crate::project::ItemState;
use crate::store::ItemRunRow;

const PAUSED: Readiness = Readiness {
    driven: true,
    slots_full: false,
    paused: true,
};

const SLOTS_FULL: Readiness = Readiness {
    driven: true,
    slots_full: true,
    paused: false,
};

fn row(step: &str, kind: &str, step_state: &str, execution_state: Option<&str>) -> ItemRunRow {
    ItemRunRow {
        run: "run".to_owned(),
        key: "A".to_owned(),
        run_state: "active".to_owned(),
        merged_as: None,
        step_key: Some(step.to_owned()),
        step_kind: Some(kind.to_owned()),
        step_state: Some(step_state.to_owned()),
        iteration: None,
        attempt: execution_state.map(|_| 1),
        execution_state: execution_state.map(str::to_owned),
        started_at: None,
    }
}

#[test]
fn while_paused_only_a_task_queued_for_an_agent_launch_names_its_step() {
    let table = [
        (
            "an agent to launch again after a transient failure",
            row("build", "agent", "running", Some("failed")),
            Some("build"),
        ),
        (
            "an agent step not started yet",
            row("review", "agent", "pending", None),
            Some("review"),
        ),
        (
            "a finished agent step",
            row("review", "agent", "done", Some("succeeded")),
            None,
        ),
        (
            "an interrupted Operation",
            row("check", "operation", "running", Some("interrupted")),
            None,
        ),
    ];

    for (case, row, step) in table {
        assert_eq!(
            standing(&row, &[], PAUSED),
            (ItemState::Queued, step),
            "{case}"
        );
    }
}

#[test]
fn a_task_waiting_for_an_agent_slot_names_only_a_step_not_started_yet() {
    let table = [
        (
            "an agent to launch again",
            row("build", "agent", "running", Some("failed")),
            None,
        ),
        (
            "an agent step not started yet",
            row("review", "agent", "pending", None),
            Some("review"),
        ),
    ];

    for (case, row, step) in table {
        assert_eq!(
            standing(&row, &[], SLOTS_FULL),
            (ItemState::Queued, step),
            "{case}"
        );
    }
}
