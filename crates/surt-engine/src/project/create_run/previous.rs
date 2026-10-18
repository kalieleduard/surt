use super::{ArtifactFile, InitialArtifact};
use crate::artifacts::working_copy::unrecorded_edits;
use crate::project::{Error, Layout, Refusal};
use crate::store::{RootRunRow, Store};

const ACTIVE: &str = "active";
const CHECKING: &str = "check the working copy of";

pub(super) async fn carried(
    store: &Store,
    layout: &Layout,
    subject: &str,
    names: &[String],
) -> Result<Vec<InitialArtifact>, Error> {
    let runs = store.root_runs(subject).await?;
    let Some(latest) = runs.last() else {
        return match names {
            [] => Ok(Vec::new()),
            _ => Err(Refusal::UnknownSubject {
                reference: subject.to_owned(),
            }
            .into()),
        };
    };
    if runs.iter().any(|run| run.state == ACTIVE) {
        return Err(Refusal::ActiveRunExists {
            subject: subject.to_owned(),
        }
        .into());
    }
    refuse_unrecorded_edits(store, layout, latest).await?;
    let mut artifacts = Vec::new();
    for name in names {
        artifacts.push(latest_revision(store, latest, name).await?);
    }
    Ok(artifacts)
}

async fn refuse_unrecorded_edits(
    store: &Store,
    layout: &Layout,
    run: &RootRunRow,
) -> Result<(), Error> {
    let files: Vec<_> = store
        .working_copy(&run.id)
        .await?
        .into_iter()
        .flat_map(|artifact| artifact.files)
        .collect();
    let dir = layout.working_copy(&run.subject);
    let edited =
        unrecorded_edits(&dir, &files).map_err(|error| Error::io(CHECKING, &dir, error))?;
    if edited.is_empty() {
        return Ok(());
    }
    Err(Refusal::UnrecordedEditsBlockNewRun {
        subject: run.subject.clone(),
        run: run.ordinal(),
        files: edited,
    }
    .into())
}

async fn latest_revision(
    store: &Store,
    run: &RootRunRow,
    name: &str,
) -> Result<InitialArtifact, Error> {
    let Some(revision) = store.revision_id(&run.id, name, None).await? else {
        let known = store.artifact_summaries(&run.id).await?;
        return Err(Refusal::UnknownArtifact {
            subject: run.subject.clone(),
            artifact: name.to_owned(),
            known: known.into_iter().map(|artifact| artifact.name).collect(),
        }
        .into());
    };
    let stored = store.revision(&revision).await?;
    Ok(InitialArtifact {
        name: name.to_owned(),
        files: stored
            .files
            .into_iter()
            .map(|(rel_path, bytes)| ArtifactFile { rel_path, bytes })
            .collect(),
    })
}
