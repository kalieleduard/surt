mod reported;

use std::path::PathBuf;

use super::super::resolving::resolve_execution_in;
use super::super::{Error, ExecutionState, Project};
use crate::drive::CorruptSnapshot;
use crate::git;
use crate::store::{ExecutionDetailRow, ExecutionInputRow, Store, number};
use crate::workflow::AgentContract;

pub use reported::{ConversationInfo, RuntimeInfo, Usage};

const SALVAGE_REFS: &str = "refs/surt/salvage/";

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionDetail {
    pub id: String,
    pub subject: String,
    pub run: u32,
    pub item: Option<String>,
    pub step: String,
    pub iteration: Option<u32>,
    pub attempt: u32,
    pub performer: Performer,
    pub state: ExecutionState,
    pub outcome: Option<String>,
    pub failure: Option<ExecutionFailure>,
    pub slot: Option<u32>,
    pub pid: Option<u32>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub reported: RuntimeInfo,
    pub inputs: Vec<ExecutionInput>,
    pub worktree: Option<PathBuf>,
    pub salvage_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Performer {
    Agent { role: String, profile: AgentProfile },
    Operation { id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentProfile {
    pub runtime: String,
    pub model: String,
    pub effort: String,
    pub permission_mode: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionFailure {
    pub class: String,
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionInput {
    pub name: String,
    pub artifact: String,
    pub revision: u32,
    pub sha256: String,
    pub item: Option<String>,
}

impl Project {
    pub async fn inspect_execution(&self, reference: &str) -> Result<ExecutionDetail, Error> {
        let store = self.reader().await?;
        let detail = read_detail(&store, reference).await;
        store.close().await;
        let mut detail = detail?;
        detail.salvage_ref = self.salvage_ref(&detail.id).await?;
        Ok(detail)
    }

    async fn salvage_ref(&self, execution: &str) -> Result<Option<String>, Error> {
        let name = format!("{SALVAGE_REFS}{execution}");
        let refs = git::refs_under(self.layout.root(), &name).await?;
        Ok(refs
            .into_iter()
            .map(|(found, _)| found)
            .find(|found| *found == name))
    }
}

async fn read_detail(store: &Store, reference: &str) -> Result<ExecutionDetail, Error> {
    let id = resolve_execution_in(store, reference).await?;
    let row = store
        .execution_detail(&id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    let inputs = store.execution_inputs(&id).await?;
    detail(row, inputs)
}

fn detail(
    row: ExecutionDetailRow,
    inputs: Vec<ExecutionInputRow>,
) -> Result<ExecutionDetail, Error> {
    let performer = performer(&row)?;
    let scope = row.scope;
    Ok(ExecutionDetail {
        id: scope.id,
        subject: scope.subject,
        run: number(scope.ordinal),
        item: scope.item,
        step: scope.step,
        iteration: scope.iteration.map(number),
        attempt: number(scope.attempt),
        performer,
        state: ExecutionState::from_column(&scope.state),
        outcome: row.outcome,
        failure: row.failure_class.map(|class| ExecutionFailure {
            class,
            code: row.error_code,
            message: row.error_message,
        }),
        slot: row.slot.map(number),
        pid: row.pid.map(number),
        started_at: row.started_at,
        finished_at: row.finished_at,
        reported: RuntimeInfo::parse(row.runtime_info.as_deref()),
        inputs: inputs.into_iter().map(ExecutionInput::from).collect(),
        worktree: row.worktree.map(PathBuf::from),
        salvage_ref: None,
    })
}

fn performer(row: &ExecutionDetailRow) -> Result<Performer, Error> {
    if let Some(id) = &row.operation {
        return Ok(Performer::Operation { id: id.clone() });
    }
    let role = row.role.clone().unwrap_or_default();
    let corrupt = |detail: String| Error::internal(CorruptSnapshot::new(&role, detail));
    let text = row
        .contract
        .as_deref()
        .map(String::from_utf8_lossy)
        .ok_or_else(|| corrupt("agent.toml is missing".to_owned()))?;
    let contract = AgentContract::frozen(&text).map_err(|error| corrupt(error.to_string()))?;
    Ok(Performer::Agent {
        profile: AgentProfile {
            runtime: contract.runtime().to_owned(),
            model: contract.model().to_owned(),
            effort: contract.effort().to_owned(),
            permission_mode: contract.permission_mode().map(str::to_owned),
        },
        role,
    })
}

impl From<ExecutionInputRow> for ExecutionInput {
    fn from(row: ExecutionInputRow) -> ExecutionInput {
        ExecutionInput {
            name: row.name,
            artifact: row.artifact,
            revision: number(row.revision),
            sha256: row.sha256,
            item: row.item,
        }
    }
}
