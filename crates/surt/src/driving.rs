mod blocked;
mod cleanup;
mod interruption;
mod progress;

use surt_engine::project::{DriveOutcome, Gate, GateStatus, Project, RunId};
use surt_engine::runtime::{AgentRuntime, StopSignal};
use tokio::sync::mpsc;

use crate::failure::Failure;
use crate::message::{Mark, Message};
use crate::refusal;
pub(crate) use cleanup::report as report_cleanup;
use interruption::Interruptions;
pub(crate) use progress::print as print_progress;

pub(crate) use blocked::{last_error, local_time, reason};
pub(crate) use cleanup::kept_line;
pub(crate) use progress::{label, short};

pub(crate) struct Driven<'a> {
    pub(crate) project: &'a Project,
    pub(crate) subject: &'a str,
    pub(crate) run: &'a RunId,
    pub(crate) command: &'static str,
}

impl Driven<'_> {
    pub(crate) async fn drive(&self, runtime: &impl AgentRuntime) -> Result<(), Failure> {
        let (sink, events) = mpsc::unbounded_channel();
        let stop = StopSignal::new();
        let relay = tokio::spawn(interruption::relay(Interruptions::listen()?, stop.clone()));
        let (outcome, cleaned) = tokio::join!(
            self.project.drive(self.run, runtime, sink, stop),
            progress::print(events)
        );
        relay.abort();
        let concluded = self.conclude(outcome?).await;
        if let Some(cleanup) = cleaned {
            cleanup::report(&cleanup, self.resume_command());
        }
        concluded
    }

    async fn conclude(&self, outcome: DriveOutcome) -> Result<(), Failure> {
        match outcome {
            DriveOutcome::AtGate { gates } => self.awaiting(&gates).await,
            DriveOutcome::Succeeded => self.succeeded().await,
            DriveOutcome::Blocked { .. } => {
                let steps = self.project.blocked_steps(self.run).await?;
                Err(Failure::Blocked(blocked::report(
                    &steps,
                    self.resume_command(),
                )))
            }
            DriveOutcome::Cancelled => Err(Failure::Cancelled(
                Message::new("cancelled by surt cancel (from another terminal)")
                    .way("inspect", format!("surt status {}", self.subject)),
            )),
            DriveOutcome::AlreadyCancelled => {
                Message::new(format!("{} was already cancelled", self.subject)).print_success();
                Ok(())
            }
            DriveOutcome::Interrupted => Err(Failure::Interrupted(
                Message::new("interrupted").way("resume", self.resume_command()),
            )),
            DriveOutcome::Halted(refusal) => Err(Failure::Refused(refusal::message(&refusal))),
        }
    }

    async fn succeeded(&self) -> Result<(), Failure> {
        if let Some(branch) = self.project.integration_branch(self.run).await? {
            Message::new(format!(
                "shipper delivered — {} @ {}",
                branch.name,
                short(&branch.tip)
            ))
            .print_success();
        }
        Message::new("run succeeded").print_success();
        Ok(())
    }

    fn resume_command(&self) -> String {
        format!("surt {} {}", self.command, self.subject)
    }

    async fn awaiting(&self, gates: &[Gate]) -> Result<(), Failure> {
        let subject = self.subject;
        for gate in gates {
            let revision = match self.project.gate_status(self.run, &gate.step.key).await? {
                GateStatus::Waiting { revision } => format!(" rev {revision}"),
                _ => String::new(),
            };
            Message::new(format!(
                "{}{revision} awaiting approval in .surt/features/{subject}",
                gate.artifact
            ))
            .way("approve", format!("surt approve {subject}"))
            .way("reject", format!("surt reject {subject} -m \"<reason>\""))
            .print(Mark::Paused);
        }
        Ok(())
    }
}
