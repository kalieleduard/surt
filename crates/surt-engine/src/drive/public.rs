use super::decide;
use super::driver::Driver;
use super::frozen::CorruptSnapshot;
use super::graph::StepRunId;
use super::load::{Loaded, label};
use super::outcome::{DriveOutcome, Gate};
use crate::runtime::AgentRuntime;
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
                    public.push(Gate {
                        step: label(row),
                        artifact: gate_artifact(self.definition(loaded, row)?, &row.key),
                    });
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

fn gate_artifact(definition: &Definition, key: &str) -> String {
    match definition.step(key).map(|step| &step.spec) {
        Some(StepSpec::Approval(gate)) => gate.artifact.clone(),
        _ => String::new(),
    }
}
