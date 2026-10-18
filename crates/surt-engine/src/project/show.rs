use serde_json::Value;

use super::status::origin;
use super::{Error, Project, Refusal, RevisionOrigin};
use crate::artifacts::working_copy;
use crate::store::{Store, StoredRevision};

const HUMAN: &str = "human";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowRequest<'a> {
    pub artifact: &'a str,
    pub revision: Option<u32>,
    pub task: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub artifact: String,
    pub revision: u32,
    pub origin: RevisionOrigin,
    pub content: Vec<u8>,
    pub edited: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactSummary {
    pub name: String,
    pub revisions: u32,
    pub latest: u32,
    pub origin: RevisionOrigin,
    pub approved: bool,
    pub files: u32,
}

impl Project {
    pub async fn artifacts(&self, reference: &str) -> Result<Vec<ArtifactSummary>, Error> {
        let subject = self.resolve_subject(reference).await?.name;
        let store = self.reader().await?;
        let summaries = summaries(&store, &subject).await;
        store.close().await;
        summaries
    }

    pub async fn show(&self, reference: &str, request: ShowRequest<'_>) -> Result<Shown, Error> {
        let subject = self.resolve_subject(reference).await?.name;
        let store = self.reader().await?;
        let shown = self.read_revision(&store, &subject, request).await;
        store.close().await;
        shown
    }

    async fn read_revision(
        &self,
        store: &Store,
        subject: &str,
        request: ShowRequest<'_>,
    ) -> Result<Shown, Error> {
        let root = root_run(store, subject).await?;
        let run = match request.task {
            None => root.clone(),
            Some(task) => task_run(store, subject, &root, task).await?,
        };
        let id = revision_id(store, subject, &run, request).await?;
        let revision = store.revision(&id).await?;
        let edited = match request.task {
            None => {
                self.unrecorded_edits(store, subject, &root, request.artifact)
                    .await?
            }
            Some(_) => Vec::new(),
        };
        Ok(Shown {
            artifact: request.artifact.to_owned(),
            revision: revision.number,
            origin: origin(&revision.producer),
            content: primary(revision),
            edited,
        })
    }

    async fn unrecorded_edits(
        &self,
        store: &Store,
        subject: &str,
        root: &str,
        artifact: &str,
    ) -> Result<Vec<String>, Error> {
        let dir = self.layout.working_copy(subject);
        let copies = store.working_copy(root).await?;
        let Some(copy) = copies
            .iter()
            .find(|copy| copy.name == artifact && !copy.pending)
        else {
            return Ok(Vec::new());
        };
        working_copy::unrecorded_edits(&dir, &copy.files)
            .map_err(|error| Error::io("read", &dir, error))
    }
}

async fn summaries(store: &Store, subject: &str) -> Result<Vec<ArtifactSummary>, Error> {
    let root = root_run(store, subject).await?;
    Ok(store
        .artifact_summaries(&root)
        .await?
        .into_iter()
        .map(|row| ArtifactSummary {
            name: row.name,
            revisions: count(row.revisions),
            latest: count(row.number),
            origin: if row.origin == HUMAN {
                RevisionOrigin::Human
            } else {
                RevisionOrigin::Agent
            },
            approved: row.approved,
            files: count(row.files),
        })
        .collect())
}

async fn root_run(store: &Store, subject: &str) -> Result<String, Error> {
    let runs = store.runs_to_show(Some(subject), true).await?;
    runs.into_iter().next().map(|run| run.id).ok_or_else(|| {
        Refusal::UnknownSubject {
            reference: subject.to_owned(),
        }
        .into()
    })
}

async fn task_run(store: &Store, subject: &str, root: &str, task: &str) -> Result<String, Error> {
    if let Some(run) = store.task_run(root, task).await? {
        return Ok(run);
    }
    Err(Refusal::UnknownTask {
        subject: subject.to_owned(),
        task: task.to_owned(),
        known: store
            .tasks(root)
            .await?
            .into_iter()
            .map(|row| row.key)
            .collect(),
    }
    .into())
}

async fn revision_id(
    store: &Store,
    subject: &str,
    run: &str,
    request: ShowRequest<'_>,
) -> Result<String, Error> {
    let artifact = request.artifact;
    if let Some(id) = store.revision_id(run, artifact, request.revision).await? {
        return Ok(id);
    }
    let refusal = match (
        store.latest_revision_number(run, artifact).await?,
        request.revision,
    ) {
        (Some(latest), Some(revision)) => Refusal::UnknownRevision {
            subject: subject.to_owned(),
            artifact: artifact.to_owned(),
            revision,
            latest,
        },
        _ => Refusal::UnknownArtifact {
            subject: subject.to_owned(),
            artifact: artifact.to_owned(),
            known: store
                .artifact_summaries(run)
                .await?
                .into_iter()
                .map(|row| row.name)
                .collect(),
        },
    };
    Err(refusal.into())
}

fn primary(revision: StoredRevision) -> Vec<u8> {
    let mut files = revision.files;
    if files.is_empty() {
        return Vec::new();
    }
    let index = files
        .iter()
        .position(|(_, bytes)| matches!(serde_json::from_slice(bytes), Ok(Value::Array(_))))
        .unwrap_or_default();
    files.swap_remove(index).1
}

fn count(value: i64) -> u32 {
    u32::try_from(value).unwrap_or_default()
}
