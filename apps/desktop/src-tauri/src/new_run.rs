use jiff::Zoned;
use serde::Serialize;
use surt_engine::project::{
    Carry, DeclaredInput, FreeName, NewRun, Project, Slug, WorkflowDefinitionSummary,
};
use tauri::State;

use crate::drivers::Drivers;
use crate::workspace::{Failure, Workspace};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowView {
    name: String,
    run_inputs: Option<Vec<RunInputView>>,
    problem: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunInputView {
    name: String,
    format: &'static str,
    optional: bool,
    default: Option<String>,
    description: Option<String>,
}

#[tauri::command]
pub async fn workflow_definitions(
    workspace: State<'_, Workspace>,
) -> Result<Vec<WorkflowView>, Failure> {
    let project = workspace.project().await?;
    Ok(project
        .workflow_definitions()?
        .into_iter()
        .map(WorkflowView::from)
        .collect())
}

#[tauri::command]
pub async fn new_run(
    workflow: String,
    inputs: Vec<(String, String)>,
    name: Option<String>,
    workspace: State<'_, Workspace>,
    drivers: State<'_, Drivers>,
) -> Result<String, Failure> {
    let project = workspace.writable().await?;
    project.subject_input(&workflow, &inputs)?;
    let subject = free_name(&project, name.as_deref()).await?.name;
    let new_run = NewRun {
        subject: subject.clone(),
        workflow,
        inputs,
        carry: Carry::Nothing,
        merge_into_base: None,
        actor: project.actor().await?,
    };
    let created = project.create_run(new_run).await?;
    drivers.start(&subject, project, created.run)?;
    Ok(subject)
}

async fn free_name(project: &Project, name: Option<&str>) -> Result<FreeName, Failure> {
    let today = Zoned::now().date();
    let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) else {
        return Ok(project.next_numbered_subject_name(today).await?);
    };
    let slug = Slug::derive(name).ok_or_else(|| Failure::NoSlug(name.to_owned()))?;
    Ok(project.free_subject_name(&slug, today).await?)
}

impl From<WorkflowDefinitionSummary> for WorkflowView {
    fn from(summary: WorkflowDefinitionSummary) -> WorkflowView {
        let (run_inputs, problem) = match summary.run_inputs {
            Ok(inputs) => (
                Some(inputs.into_iter().map(RunInputView::from).collect()),
                None,
            ),
            Err(error) => (None, Some(error.to_string())),
        };
        WorkflowView {
            name: summary.name,
            run_inputs,
            problem,
        }
    }
}

impl From<DeclaredInput> for RunInputView {
    fn from(input: DeclaredInput) -> RunInputView {
        RunInputView {
            name: input.name,
            format: input.format.name(),
            optional: input.optional,
            default: input.default,
            description: input.description,
        }
    }
}

#[cfg(test)]
mod tests;
