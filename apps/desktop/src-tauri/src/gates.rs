use std::collections::BTreeMap;

use serde::Serialize;
use surt_engine::project::{Choice, Question, RunId, Subject, WaitingInput};
use tauri::State;

use crate::drivers::Drivers;
use crate::workspace::{Failure, Workspace};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingInputView {
    step: String,
    artifact: String,
    revision: u32,
    round: u32,
    questions: Vec<QuestionView>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuestionView {
    id: String,
    question: String,
    why: String,
    recommended: String,
    options: Option<Vec<ChoiceView>>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ChoiceView {
    value: String,
    description: Option<String>,
}

#[tauri::command]
pub async fn approve(
    subject: String,
    revision: Option<u32>,
    workspace: State<'_, Workspace>,
    drivers: State<'_, Drivers>,
) -> Result<(), Failure> {
    let project = workspace.writable().await?;
    let found = project.resolve_subject(&subject).await?;
    let run = active_run(&found)?;
    let actor = project.actor().await?;
    project.approve(&run, revision, &actor).await?;
    drivers.start(&found.name, project, run)
}

#[tauri::command]
pub async fn reject(
    subject: String,
    revision: Option<u32>,
    reason: String,
    workspace: State<'_, Workspace>,
    drivers: State<'_, Drivers>,
) -> Result<(), Failure> {
    let project = workspace.writable().await?;
    let found = project.resolve_subject(&subject).await?;
    let run = active_run(&found)?;
    let actor = project.actor().await?;
    project.reject(&run, revision, &reason, &actor).await?;
    drivers.start(&found.name, project, run)
}

#[tauri::command]
pub async fn waiting_input(
    subject: String,
    workspace: State<'_, Workspace>,
) -> Result<WaitingInputView, Failure> {
    let project = workspace.project().await?;
    let found = project.resolve_subject(&subject).await?;
    let run = active_run(&found)?;
    Ok(WaitingInputView::from(project.waiting_input(&run).await?))
}

#[tauri::command]
pub async fn answer(
    subject: String,
    revision: u32,
    answers: BTreeMap<String, String>,
    note: Option<String>,
    workspace: State<'_, Workspace>,
    drivers: State<'_, Drivers>,
) -> Result<(), Failure> {
    let project = workspace.writable().await?;
    let found = project.resolve_subject(&subject).await?;
    let run = active_run(&found)?;
    let actor = project.actor().await?;
    let note = note.filter(|text| !text.trim().is_empty());
    project
        .answer(&run, revision, &answers, note.as_deref(), &actor)
        .await?;
    drivers.start(&found.name, project, run)
}

pub fn active_run(subject: &Subject) -> Result<RunId, Failure> {
    subject
        .active_run
        .clone()
        .ok_or_else(|| Failure::NoActiveRun(subject.name.clone()))
}

impl From<WaitingInput> for WaitingInputView {
    fn from(waiting: WaitingInput) -> WaitingInputView {
        WaitingInputView {
            questions: waiting
                .questions
                .questions()
                .iter()
                .map(QuestionView::from)
                .collect(),
            step: waiting.step,
            artifact: waiting.artifact,
            revision: waiting.revision,
            round: waiting.round,
        }
    }
}

impl From<&Question> for QuestionView {
    fn from(question: &Question) -> QuestionView {
        QuestionView {
            id: question.id().to_owned(),
            question: question.question().to_owned(),
            why: question.why().to_owned(),
            recommended: question.recommended().to_owned(),
            options: question
                .options()
                .map(|choices| choices.iter().map(ChoiceView::from).collect()),
        }
    }
}

impl From<&Choice> for ChoiceView {
    fn from(choice: &Choice) -> ChoiceView {
        ChoiceView {
            value: choice.value().to_owned(),
            description: choice.description().map(str::to_owned),
        }
    }
}

#[cfg(test)]
mod tests;
