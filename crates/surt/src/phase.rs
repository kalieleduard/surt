use surt_engine::project::{GateStatus, Project, RunId, Subject};
use surt_engine::runtime::AgentRuntime;

use crate::driving::Driven;
use crate::failure::Failure;
use crate::message::Message;
use crate::runtime;

pub(crate) struct Phase {
    command: &'static str,
    gate: &'static str,
    artifact: &'static str,
    agent: &'static str,
    next: &'static str,
    covers_item_files: bool,
}

pub(crate) const SPEC: Phase = Phase {
    command: "feature",
    gate: "approve-spec",
    artifact: "spec",
    agent: "Spec Agent",
    next: "plan",
    covers_item_files: false,
};

pub(crate) const TASKS: Phase = Phase {
    command: "plan",
    gate: "approve-tasks",
    artifact: "manifest",
    agent: "Task Agent",
    next: "start",
    covers_item_files: true,
};

const PHASES: [&Phase; 2] = [&SPEC, &TASKS];
const NEXT: &str = "next";

pub(crate) fn current_command(subject: &str, approved: impl Fn(&str) -> bool) -> String {
    PHASES
        .into_iter()
        .rev()
        .find(|phase| approved(phase.gate))
        .map_or_else(
            || SPEC.resume_command(subject),
            |phase| phase.next_command(subject),
        )
}

impl Phase {
    pub(crate) fn of_gate(gate: &str) -> Option<&'static Phase> {
        PHASES.into_iter().find(|phase| phase.gate == gate)
    }

    pub(crate) fn agent(&self) -> &'static str {
        self.agent
    }

    pub(crate) fn covers_item_files(&self) -> bool {
        self.covers_item_files
    }

    pub(crate) fn resume_command(&self, subject: &str) -> String {
        format!("surt {} {subject}", self.command)
    }

    pub(crate) fn next_command(&self, subject: &str) -> String {
        format!("surt {} {subject}", self.next)
    }

    pub(crate) async fn require_approved(
        &self,
        project: &Project,
        subject: &str,
        run: &RunId,
    ) -> Result<(), Failure> {
        let artifact = self.artifact;
        let (state, way) = match project.gate_status(run, self.gate).await? {
            GateStatus::Approved { .. } => return Ok(()),
            GateStatus::Waiting { revision } => (
                format!("{artifact} rev {revision} awaiting approval"),
                (
                    self.review_label(revision),
                    format!("surt approve {subject}"),
                ),
            ),
            GateStatus::Rejected { revision } => (
                format!("{artifact} rev {revision} rejected"),
                ("regenerate".to_owned(), self.resume_command(subject)),
            ),
            GateStatus::NotReached => (
                format!("no {artifact} revision yet"),
                ("resume".to_owned(), self.resume_command(subject)),
            ),
        };
        Err(Failure::Refused(
            Message::new(format!("{artifact} not approved yet ({state})")).way(&way.0, way.1),
        ))
    }

    fn review_label(&self, revision: u32) -> String {
        if self.covers_item_files {
            return format!(
                "review {} rev {revision} and its Task Markdown files, then",
                self.artifact
            );
        }
        NEXT.to_owned()
    }

    pub(crate) async fn finish_cleanup(
        &self,
        project: &Project,
        subject: &Subject,
    ) -> Result<bool, Failure> {
        finish_cleanup(project, subject, self.command).await
    }

    pub(crate) async fn resume(&self, project: &Project, subject: &Subject) -> Result<(), Failure> {
        let run = active_run(subject)?;
        if let GateStatus::Approved { revision } = project.gate_status(run, self.gate).await? {
            Message::new(format!(
                "{} rev {revision} already approved — next: {}",
                self.artifact,
                self.next_command(&subject.name)
            ))
            .print_success();
            return Ok(());
        }
        let runtime = runtime::select()?;
        self.drive(project, &subject.name, run, &runtime).await
    }

    pub(crate) async fn drive(
        &self,
        project: &Project,
        subject: &str,
        run: &RunId,
        runtime: &impl AgentRuntime,
    ) -> Result<(), Failure> {
        let driven = Driven {
            project,
            subject,
            run,
            command: self.command,
        };
        driven.drive(runtime).await
    }
}

pub(crate) async fn finish_cleanup(
    project: &Project,
    subject: &Subject,
    command: &'static str,
) -> Result<bool, Failure> {
    let (None, Some(run)) = (&subject.active_run, &subject.pending_cleanup) else {
        return Ok(false);
    };
    let runtime = runtime::select()?;
    let driven = Driven {
        project,
        subject: &subject.name,
        run,
        command,
    };
    driven.drive(&runtime).await?;
    Ok(true)
}

pub(crate) fn active_run(subject: &Subject) -> Result<&RunId, Failure> {
    subject.active_run.as_ref().ok_or_else(|| {
        Failure::Refused(
            Message::new(format!("{} has no active run", subject.name))
                .way("inspect", format!("surt status {}", subject.name)),
        )
    })
}
