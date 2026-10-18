use super::naming::without_date;
use super::{Error, GateStatus, Project, Refusal, RunId, RunState, RunStatus};
use crate::store::{Store, SubjectRow};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub name: String,
    pub active_run: Option<RunId>,
    pub pending_cleanup: Option<RunId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub states: Vec<RunState>,
    pub awaiting: Option<AwaitedRevision>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwaitedRevision {
    pub artifact: String,
    pub revision: u32,
}

impl Project {
    pub async fn subject_named(&self, name: &str) -> Result<Option<Subject>, Error> {
        let subjects = self.subjects().await?;
        Ok(subjects.into_iter().find(|subject| subject.name == name))
    }

    pub async fn resolve_subject(&self, reference: &str) -> Result<Subject, Error> {
        let subjects = self.subjects().await?;
        if let Some(exact) = subjects.iter().find(|subject| subject.name == reference) {
            return Ok(exact.clone());
        }
        let by_slug = matching(&subjects, |name| without_date(name) == Some(reference));
        let candidates = if by_slug.is_empty() {
            matching(&subjects, |name| name.starts_with(reference))
        } else {
            by_slug
        };
        match candidates.as_slice() {
            [] => Err(Refusal::UnknownSubject {
                reference: reference.to_owned(),
            }
            .into()),
            [only] => Ok((*only).clone()),
            many => Err(Refusal::AmbiguousSubject {
                reference: reference.to_owned(),
                candidates: self.candidates(many).await?,
            }
            .into()),
        }
    }

    async fn candidates(&self, subjects: &[&Subject]) -> Result<Vec<Candidate>, Error> {
        let store = self.reader().await?;
        let candidates = self.read_candidates(&store, subjects).await;
        store.close().await;
        candidates
    }

    async fn read_candidates(
        &self,
        store: &Store,
        subjects: &[&Subject],
    ) -> Result<Vec<Candidate>, Error> {
        let mut candidates = Vec::new();
        for subject in subjects {
            let mut statuses = self.read_statuses(store, Some(&subject.name), true).await?;
            candidates.push(candidate(&subject.name, statuses.pop()));
        }
        Ok(candidates)
    }

    async fn subjects(&self) -> Result<Vec<Subject>, Error> {
        let store = self.reader().await?;
        let rows = store.subjects().await;
        store.close().await;
        Ok(rows?.into_iter().map(Subject::from).collect())
    }
}

fn candidate(name: &str, status: Option<RunStatus>) -> Candidate {
    Candidate {
        name: name.to_owned(),
        awaiting: status.as_ref().and_then(|status| {
            status.gates.iter().find_map(|gate| match gate.status {
                GateStatus::Waiting { revision } => Some(AwaitedRevision {
                    artifact: gate.artifact.clone(),
                    revision,
                }),
                _ => None,
            })
        }),
        states: status.map(|status| status.states).unwrap_or_default(),
    }
}

fn matching(subjects: &[Subject], matches: impl Fn(&str) -> bool) -> Vec<&Subject> {
    subjects
        .iter()
        .filter(|subject| matches(&subject.name))
        .collect()
}

impl From<SubjectRow> for Subject {
    fn from(row: SubjectRow) -> Subject {
        Subject {
            name: row.name,
            active_run: row.active_run.map(RunId),
            pending_cleanup: row.pending_cleanup.map(RunId),
        }
    }
}
