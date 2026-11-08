use surt_engine::project::{DriveOutcome, Gate, GateKind, Refusal, StepLabel};
use tokio::sync::oneshot;

use super::{DriverView, Drivers, Ending};
use crate::workspace::Failure;

const SUBJECT: &str = "20261107-login";
const OTHER: &str = "20261107-signup";

fn view(subject: &str, driving: bool, ending: Option<Ending>) -> DriverView {
    DriverView {
        subject: subject.to_owned(),
        driving,
        ending,
    }
}

fn gate(artifact: &str, kind: GateKind) -> Gate {
    Gate {
        step: StepLabel {
            key: format!("decide-{artifact}"),
            item: None,
            iteration: Some(1),
        },
        kind,
        artifact: artifact.to_owned(),
    }
}

#[tokio::test]
async fn a_drive_that_ends_frees_the_subject_and_keeps_its_ending() {
    let drivers = Drivers::default();
    let reached = Ending::new("at_gate", Some("spec awaits approval".to_owned()));
    let ending = reached.clone();

    drivers
        .start_with(SUBJECT, move |_| async move { ending })
        .expect("the drive starts");
    drivers.stop_all().await;

    assert!(!drivers.is_driving());
    assert_eq!(drivers.views(), [view(SUBJECT, false, Some(reached))]);
    drivers
        .start_with(SUBJECT, |_| async { Ending::new("succeeded", None) })
        .expect("an ended subject can be driven again");
    drivers.stop_all().await;
}

#[tokio::test]
async fn a_subject_already_driven_from_this_window_is_refused_a_second_driver() {
    let drivers = Drivers::default();
    let (release, released) = oneshot::channel::<()>();
    drivers
        .start_with(SUBJECT, move |_| async move {
            let _ = released.await;
            Ending::new("succeeded", None)
        })
        .expect("the first drive starts");

    let second = drivers.start_with(SUBJECT, |_| async { Ending::new("succeeded", None) });

    assert!(matches!(second, Err(Failure::AlreadyDriving(subject)) if subject == SUBJECT));
    drivers
        .start_with(OTHER, |_| async { Ending::new("succeeded", None) })
        .expect("another subject drives alongside");
    assert!(drivers.is_driving());
    assert_eq!(release.send(()), Ok(()));
    drivers.stop_all().await;
    assert_eq!(
        drivers.views(),
        [
            view(SUBJECT, false, Some(Ending::new("succeeded", None))),
            view(OTHER, false, Some(Ending::new("succeeded", None))),
        ]
    );
}

#[tokio::test]
async fn stopping_every_driver_asks_each_drive_to_stop_and_waits_for_it() {
    let drivers = Drivers::default();
    drivers
        .start_with(SUBJECT, |stop| async move {
            stop.requested().await;
            Ending::new("interrupted", None)
        })
        .expect("the drive starts");
    assert_eq!(drivers.views(), [view(SUBJECT, true, None)]);

    drivers.stop_all().await;

    assert_eq!(
        drivers.views(),
        [view(SUBJECT, false, Some(Ending::new("interrupted", None)))]
    );
}

#[test]
fn a_drive_that_reaches_gates_names_what_each_one_awaits() {
    let ending = Ending::of(Ok(DriveOutcome::AtGate {
        gates: vec![
            gate("spec", GateKind::Approval),
            gate("spec-questions", GateKind::Input),
        ],
    }));

    assert_eq!(
        ending,
        Ending::new(
            "at_gate",
            Some("spec awaits approval, spec-questions awaits answers".to_owned())
        )
    );
}

#[test]
fn a_drive_that_cannot_go_on_carries_the_reason_to_show() {
    let halted = Ending::of(Ok(DriveOutcome::Halted(Refusal::UnknownSubject {
        reference: SUBJECT.to_owned(),
    })));
    let failed = Ending::of(Err(Failure::NoActiveRun(SUBJECT.to_owned())));

    assert_eq!(halted.outcome, "refused");
    assert!(
        halted
            .message
            .as_deref()
            .is_some_and(|message| message.contains(SUBJECT)),
        "{halted:?}"
    );
    assert_eq!(
        failed,
        Ending::new("failed", Some(format!("{SUBJECT} has no active run")))
    );
}

#[test]
fn every_other_drive_outcome_maps_to_the_state_it_leaves_the_run_in() {
    let outcomes = [
        (DriveOutcome::Succeeded, "succeeded"),
        (DriveOutcome::Blocked { steps: Vec::new() }, "blocked"),
        (DriveOutcome::Cancelled, "cancelled"),
        (DriveOutcome::AlreadyCancelled, "cancelled"),
        (DriveOutcome::Interrupted, "interrupted"),
    ];

    for (outcome, name) in outcomes {
        assert_eq!(Ending::of(Ok(outcome)), Ending::new(name, None));
    }
}
