mod picking;

pub(super) use picking::item_run;
use picking::{content, primary, revision_id};

use super::status::origin;
use super::{Error, Project, RevisionOrigin, RunSelector};
use crate::artifacts::working_copy;
use crate::store::{ApprovalRow, ArtifactSummaryRow, Store, number};

const HUMAN: &str = "human";
const APPROVE: &str = "approve";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowRequest<'a> {
    pub artifact: &'a str,
    pub revision: Option<u32>,
    pub item: Option<&'a str>,
    pub iteration: Option<u32>,
    pub file: Option<&'a str>,
    pub run: RunSelector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub artifact: String,
    pub revision: u32,
    pub origin: RevisionOrigin,
    pub sha256: String,
    pub approval: Option<RevisionApproval>,
    pub content: Vec<u8>,
    pub files: Vec<ShownFile>,
    pub edited: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionApproval {
    pub approved: bool,
    pub actor: String,
    pub decided_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShownFile {
    pub path: String,
    pub content: Vec<u8>,
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
    pub async fn artifacts(
        &self,
        reference: &str,
        run: RunSelector,
    ) -> Result<Vec<ArtifactSummary>, Error> {
        let subject = self.resolve_subject(reference).await?.name;
        let store = self.reader().await?;
        let summaries = self.summaries(&store, &subject, run).await;
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
        let root = self.select_run(store, subject, request.run).await?.id;
        let run = match request.item {
            None => root.clone(),
            Some(item) => item_run(store, subject, &root, item).await?,
        };
        let id = revision_id(store, subject, &run, request).await?;
        let revision = store.revision(&id).await?;
        let files = content(subject, request, revision.number, revision.files)?;
        Ok(Shown {
            artifact: request.artifact.to_owned(),
            revision: revision.number,
            origin: origin(&revision.producer),
            sha256: revision.sha256,
            approval: store.revision_approval(&id).await?.map(approval),
            content: primary(&files),
            files: files
                .into_iter()
                .map(|(path, content)| ShownFile { path, content })
                .collect(),
            edited: self
                .unrecorded_edits(store, subject, &root, request)
                .await?,
        })
    }

    async fn unrecorded_edits(
        &self,
        store: &Store,
        subject: &str,
        root: &str,
        request: ShowRequest<'_>,
    ) -> Result<Vec<String>, Error> {
        if request.item.is_some() {
            return Ok(Vec::new());
        }
        let dir = self.layout.working_copy(subject);
        let copies = store.working_copy(root).await?;
        let Some(copy) = copies
            .iter()
            .find(|copy| copy.name == request.artifact && !copy.pending)
        else {
            return Ok(Vec::new());
        };
        let mut edited = working_copy::unrecorded_edits(&dir, &copy.files)
            .map_err(|error| Error::io("read", &dir, error))?;
        edited.retain(|path| request.file.is_none_or(|file| file == path));
        Ok(edited)
    }

    async fn summaries(
        &self,
        store: &Store,
        subject: &str,
        run: RunSelector,
    ) -> Result<Vec<ArtifactSummary>, Error> {
        let root = self.select_run(store, subject, run).await?.id;
        Ok(store
            .artifact_summaries(&root)
            .await?
            .into_iter()
            .map(summary)
            .collect())
    }
}

fn summary(row: ArtifactSummaryRow) -> ArtifactSummary {
    ArtifactSummary {
        name: row.name,
        revisions: number(row.revisions),
        latest: number(row.number),
        origin: if row.origin == HUMAN {
            RevisionOrigin::Human
        } else {
            RevisionOrigin::Agent
        },
        approved: row.approved,
        files: number(row.files),
    }
}

fn approval(row: ApprovalRow) -> RevisionApproval {
    RevisionApproval {
        approved: row.decision == APPROVE,
        actor: row.actor,
        decided_at: row.decided_at,
    }
}
