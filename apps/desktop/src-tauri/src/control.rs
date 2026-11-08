use serde::Serialize;
use surt_engine::project::{CancelImpact, Cancellation, Project, RunId, Subject};
use tauri::State;
use tokio::sync::mpsc;

use crate::drivers::Drivers;
use crate::gates::active_run;
use crate::workspace::{Failure, Workspace};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelImpactView {
    executions: usize,
    items: usize,
    merged: usize,
    integration_branch: Option<String>,
}

#[tauri::command]
pub async fn continue_run(
    subject: String,
    workspace: State<'_, Workspace>,
    drivers: State<'_, Drivers>,
) -> Result<(), Failure> {
    let project = workspace.writable().await?;
    let found = project.resolve_subject(&subject).await?;
    let run = run_to_drive(&found)?;
    drivers.start(&found.name, project, run)
}

#[tauri::command]
pub async fn stop(subject: String, workspace: State<'_, Workspace>) -> Result<(), Failure> {
    let project = workspace.writable().await?;
    let found = project.resolve_subject(&subject).await?;
    project.request_stop(&found.name).await?;
    Ok(())
}

#[tauri::command]
pub async fn cancel_impact(
    subject: String,
    workspace: State<'_, Workspace>,
) -> Result<CancelImpactView, Failure> {
    let project = workspace.project().await?;
    let found = project.resolve_subject(&subject).await?;
    let run = active_run(&found)?;
    Ok(CancelImpactView::from(project.cancel_impact(&run).await?))
}

#[tauri::command]
pub async fn cancel(
    subject: String,
    workspace: State<'_, Workspace>,
) -> Result<&'static str, Failure> {
    let project = workspace.writable().await?;
    let found = project.resolve_subject(&subject).await?;
    let run = run_to_drive(&found)?;
    Ok(cancellation_name(&cancel_run(&project, &run).await?))
}

pub fn run_to_drive(subject: &Subject) -> Result<RunId, Failure> {
    subject
        .active_run
        .clone()
        .or_else(|| subject.pending_cleanup.clone())
        .ok_or_else(|| Failure::NoActiveRun(subject.name.clone()))
}

async fn cancel_run(project: &Project, run: &RunId) -> Result<Cancellation, Failure> {
    let (sink, mut progress) = mpsc::unbounded_channel();
    let (cancellation, ()) = tokio::join!(project.cancel(run, sink), async {
        while progress.recv().await.is_some() {}
    });
    Ok(cancellation?)
}

fn cancellation_name(cancellation: &Cancellation) -> &'static str {
    match cancellation {
        Cancellation::Cancelled => "cancelled",
        Cancellation::AlreadyCancelled => "already_cancelled",
        Cancellation::AlreadySucceeded => "already_succeeded",
    }
}

impl From<CancelImpact> for CancelImpactView {
    fn from(impact: CancelImpact) -> CancelImpactView {
        CancelImpactView {
            executions: impact.executions,
            items: impact.items,
            merged: impact.merged,
            integration_branch: impact.integration_branch,
        }
    }
}
