use super::decide;
use super::driver::Driver;
use super::frozen::CorruptSnapshot;
use super::graph::StepRunId;
use super::load::{Loaded, label};
use super::outcome::{DriveOutcome, Gate, GateKind};
use crate::runtime::AgentRuntime;
use crate::store::StepRow;
use crate::workflow::{Definition, StepSpec};

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) fn public(
        &self,
        loaded: &Loaded,
        outcome: decide::DriveOutcome,
    ) -> Result<DriveOutcome, CorruptSnapshot> {
        let rows = |steps: &[StepRunId]| {
            steps
                .iter()
                .filter_map(|step| loaded.step(step))
                .collect::<Vec<_>>()
        };
        Ok(match outcome {
            decide::DriveOutcome::AtGate { gates } => {
                let mut public = Vec::new();
                for row in rows(&gates) {
                    public.push(gate(self.definition(loaded, row)?, row)?);
                }
                DriveOutcome::AtGate { gates: public }
            }
            decide::DriveOutcome::Blocked { steps } => DriveOutcome::Blocked {
                steps: rows(&steps).into_iter().map(label).collect(),
            },
            decide::DriveOutcome::Succeeded => DriveOutcome::Succeeded,
            decide::DriveOutcome::Cancelled => DriveOutcome::Cancelled,
            decide::DriveOutcome::Interrupted => DriveOutcome::Interrupted,
        })
    }
}

fn gate(definition: &Definition, row: &StepRow) -> Result<Gate, CorruptSnapshot> {
    let (kind, artifact) = match definition.step(&row.key).map(|step| &step.spec) {
        Some(StepSpec::Approval(gate)) => (GateKind::Approval, &gate.artifact),
        Some(StepSpec::Input(input)) => (GateKind::Input, &input.artifact),
        _ => return Err(CorruptSnapshot::new(&row.key, "not a gate step")),
    };
    Ok(Gate {
        step: label(row),
        kind,
        artifact: artifact.clone(),
    })
}
