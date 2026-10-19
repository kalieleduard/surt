use super::Subject;
use crate::project::{Error, ExecutionCandidate, ExecutionState, Project, Refusal};
use crate::store::{ExecutionScopeRow, Store, number};

pub(crate) const MINIMUM_PREFIX: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reference {
    Subject(Subject),
    Execution(String),
}

impl Project {
    pub async fn reference(&self, reference: &str) -> Result<Reference, Error> {
        match self.resolve_subject(reference).await {
            Err(Error::Refused(Refusal::UnknownSubject { .. })) if execution_shaped(reference) => {
                self.resolve_execution(reference)
                    .await
                    .map(Reference::Execution)
            }
            resolved => resolved.map(Reference::Subject),
        }
    }

    pub async fn resolve_execution(&self, reference: &str) -> Result<String, Error> {
        let store = self.reader().await?;
        let found = resolve(&store, reference).await;
        store.close().await;
        found
    }
}

pub(crate) async fn resolve(store: &Store, reference: &str) -> Result<String, Error> {
    let prefix = reference.to_ascii_lowercase();
    let rows = store.executions_starting_with(&prefix).await?;
    let reference = reference.to_owned();
    if rows.is_empty() {
        return Err(Refusal::UnknownExecution { reference }.into());
    }
    let candidates = rows.into_iter().map(ExecutionCandidate::from).collect();
    if prefix.chars().count() < MINIMUM_PREFIX {
        return Err(Refusal::ExecutionPrefixTooShort {
            reference,
            minimum: MINIMUM_PREFIX,
            candidates,
        }
        .into());
    }
    match <[ExecutionCandidate; 1]>::try_from(candidates) {
        Ok([only]) => Ok(only.id),
        Err(candidates) => Err(Refusal::AmbiguousExecution {
            reference,
            candidates,
        }
        .into()),
    }
}

fn execution_shaped(reference: &str) -> bool {
    !reference.is_empty()
        && reference
            .chars()
            .all(|character| character.is_ascii_hexdigit() || character == '-')
}

impl From<ExecutionScopeRow> for ExecutionCandidate {
    fn from(row: ExecutionScopeRow) -> ExecutionCandidate {
        ExecutionCandidate {
            id: row.id,
            subject: row.subject,
            run: number(row.ordinal),
            item: row.item,
            step: row.step,
            iteration: row.iteration.and_then(|value| u32::try_from(value).ok()),
            attempt: number(row.attempt),
            state: ExecutionState::from_column(&row.state),
        }
    }
}
