mod executions;

use super::naming::without_date;
use super::{
    AwaitedRevision, Candidate, Error, GateKind, GateStatus, KnownRun, Project, Refusal, RunId,
    RunStatus,
};
use crate::cleanup;
use crate::store::{RootRunRow, Store, SubjectRow};

pub use executions::Reference;
pub(crate) use executions::resolve as resolve_execution_in;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunSelector {
    #[default]
    Current,
    Ordinal(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub name: String,
    pub active_run: Option<RunId>,
    pub pending_cleanup: Option<RunId>,
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

    pub(super) async fn select_run(
        &self,
        store: &Store,
        subject: &str,
        selector: RunSelector,
    ) -> Result<RootRunRow, Error> {
        let runs = match selector {
            RunSelector::Current => store.runs_to_show(Some(subject), true).await?,
            RunSelector::Ordinal(_) => store.root_runs(subject).await?,
        };
        let found = runs.iter().find(|run| match selector {
            RunSelector::Current => true,
            RunSelector::Ordinal(ordinal) => run.ordinal() == ordinal,
        });
        match (found, selector) {
            (Some(run), _) => Ok(run.clone()),
            (None, RunSelector::Ordinal(run)) => {
                Err(self.unknown_run(store, subject, run, &runs).await?)
            }
            (None, RunSelector::Current) => Err(Refusal::UnknownSubject {
                reference: subject.to_owned(),
            }
            .into()),
        }
    }

    async fn unknown_run(
        &self,
        store: &Store,
        subject: &str,
        run: u32,
        runs: &[RootRunRow],
    ) -> Result<Error, Error> {
        let mut known = Vec::new();
        for row in runs {
            let status = self.run_status(store, row).await?;
            known.push(KnownRun {
                run: status.run,
                states: status.states,
            });
        }
        Ok(Refusal::UnknownRun {
            subject: subject.to_owned(),
            run,
            runs: known,
        }
        .into())
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
        let subjects = read_subjects(&store).await;
        store.close().await;
        subjects
    }
}

fn candidate(name: &str, status: Option<RunStatus>) -> Candidate {
    Candidate {
        name: name.to_owned(),
        awaiting: status.as_ref().and_then(|status| {
            status.gates.iter().find_map(|gate| match gate.status {
                GateStatus::Waiting { revision } if gate.kind == GateKind::Approval => {
                    Some(AwaitedRevision {
                        artifact: gate.artifact.clone(),
                        revision,
                    })
                }
                _ => None,
            })
        }),
        states: status.map(|status| status.states).unwrap_or_default(),
    }
}

async fn read_subjects(store: &Store) -> Result<Vec<Subject>, Error> {
    let mut subjects = Vec::new();
    for row in store.subjects().await? {
        let mut subject = Subject::from(row);
        if let Some(run) = &subject.pending_cleanup
            && cleanup::is_kept(store, run).await?
        {
            subject.pending_cleanup = None;
        }
        subjects.push(subject);
    }
    Ok(subjects)
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
