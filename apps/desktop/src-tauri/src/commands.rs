use std::path::PathBuf;

use surt_engine::project::{InspectRequest, Project, RunSelector, StatusScope};
use tauri::State;

use crate::definition::RunDetail;
use crate::drivers::Drivers;
use crate::status::{ProjectView, RunsView};
use crate::workspace::{Failure, Workspace};

#[tauri::command]
pub async fn current_project(
    workspace: State<'_, Workspace>,
) -> Result<Option<ProjectView>, Failure> {
    if !workspace.has_project() {
        return Ok(None);
    }
    let project = workspace.project().await?;
    Ok(Some(ProjectView::of(&project)))
}

#[tauri::command]
pub async fn open_project(
    path: PathBuf,
    workspace: State<'_, Workspace>,
) -> Result<ProjectView, Failure> {
    let project = Project::open_read_only(&path).await?;
    workspace.select(project.root().to_path_buf());
    Ok(ProjectView::of(&project))
}

#[tauri::command]
pub async fn runs(
    workspace: State<'_, Workspace>,
    drivers: State<'_, Drivers>,
) -> Result<RunsView, Failure> {
    let project = workspace.project().await?;
    let statuses = project.status(StatusScope::All).await?;
    Ok(RunsView::of(&project, statuses, drivers.views()))
}

#[tauri::command]
pub async fn run_detail(
    subject: String,
    item: Option<String>,
    workspace: State<'_, Workspace>,
) -> Result<RunDetail, Failure> {
    let project = workspace.project().await?;
    let request = InspectRequest {
        item: item.as_deref(),
        run: RunSelector::Current,
    };
    let definition = project.frozen_definition(&subject, request).await?;
    let timeline = project.inspect(&subject, request).await?;
    Ok(RunDetail::of(definition, timeline))
}
