use super::naming::without_date;
use super::{Error, Project, Refusal, RunId, open_store};
use crate::store::SubjectRow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub name: String,
    pub active_run: Option<RunId>,
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
                candidates: many.iter().map(|subject| subject.name.clone()).collect(),
            }
            .into()),
        }
    }

    async fn subjects(&self) -> Result<Vec<Subject>, Error> {
        let store = open_store(&self.layout).await?;
        let rows = store.subjects().await;
        store.close().await;
        Ok(rows?.into_iter().map(Subject::from).collect())
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
        }
    }
}
