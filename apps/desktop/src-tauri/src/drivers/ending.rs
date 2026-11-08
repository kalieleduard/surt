use serde::Serialize;
use surt_engine::project::{DriveOutcome, Error, Gate, GateKind, Project, RunId};
use surt_engine::runtime::StopSignal;
use tokio::sync::mpsc;

use crate::runtime;
use crate::workspace::Failure;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Ending {
    pub outcome: &'static str,
    pub message: Option<String>,
}

impl Ending {
    pub fn new(outcome: &'static str, message: Option<String>) -> Ending {
        Ending { outcome, message }
    }

    pub fn of(driven: Result<DriveOutcome, Failure>) -> Ending {
        match driven {
            Ok(DriveOutcome::AtGate { gates }) => Ending::new("at_gate", Some(awaiting(&gates))),
            Ok(DriveOutcome::Succeeded) => Ending::new("succeeded", None),
            Ok(DriveOutcome::Blocked { .. }) => Ending::new("blocked", None),
            Ok(DriveOutcome::Cancelled | DriveOutcome::AlreadyCancelled) => {
                Ending::new("cancelled", None)
            }
            Ok(DriveOutcome::Interrupted) => Ending::new("interrupted", None),
            Ok(DriveOutcome::Halted(refusal)) => {
                Ending::new("refused", Some(Error::Refused(refusal).to_string()))
            }
            Err(failure) => Ending::new("failed", Some(failure.to_string())),
        }
    }
}

pub(super) async fn drive(project: &Project, run: &RunId, stop: StopSignal) -> Ending {
    Ending::of(drive_once(project, run, stop).await)
}

async fn drive_once(
    project: &Project,
    run: &RunId,
    stop: StopSignal,
) -> Result<DriveOutcome, Failure> {
    let runtime = runtime::select(project)?;
    let (sink, mut progress) = mpsc::unbounded_channel();
    let (outcome, ()) = tokio::join!(project.drive(run, &runtime, sink, stop), async {
        while progress.recv().await.is_some() {}
    });
    Ok(outcome?)
}

fn awaiting(gates: &[Gate]) -> String {
    gates
        .iter()
        .map(|gate| match gate.kind {
            GateKind::Approval => format!("{} awaits approval", gate.artifact),
            GateKind::Input => format!("{} awaits answers", gate.artifact),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
