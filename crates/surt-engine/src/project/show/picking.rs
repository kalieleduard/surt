use serde_json::Value;

use super::ShowRequest;
use crate::project::{Error, Refusal};
use crate::store::Store;

pub(super) async fn item_run(
    store: &Store,
    subject: &str,
    root: &str,
    item: &str,
) -> Result<String, Error> {
    if let Some(run) = store.item_run(root, item).await? {
        return Ok(run);
    }
    Err(Refusal::UnknownItem {
        subject: subject.to_owned(),
        item: item.to_owned(),
        known: store
            .item_runs(root)
            .await?
            .into_iter()
            .map(|row| row.key)
            .collect(),
    }
    .into())
}

pub(super) async fn revision_id(
    store: &Store,
    subject: &str,
    run: &str,
    request: ShowRequest<'_>,
) -> Result<String, Error> {
    let artifact = request.artifact;
    let found = match request.iteration {
        Some(iteration) => {
            store
                .revision_in_iteration(run, artifact, iteration)
                .await?
        }
        None => store.revision_id(run, artifact, request.revision).await?,
    };
    match found {
        Some(id) => Ok(id),
        None => Err(missing(store, subject, run, request).await?.into()),
    }
}

async fn missing(
    store: &Store,
    subject: &str,
    run: &str,
    request: ShowRequest<'_>,
) -> Result<Refusal, Error> {
    let artifact = request.artifact;
    let latest = store.latest_revision_number(run, artifact).await?;
    Ok(match (latest, request.iteration, request.revision) {
        (Some(_), Some(iteration), _) => Refusal::NoRevisionInIteration {
            subject: subject.to_owned(),
            item: request.item.map(str::to_owned),
            artifact: artifact.to_owned(),
            iteration,
            iterations: store.revision_iterations(run, artifact).await?,
        },
        (Some(latest), None, Some(revision)) => Refusal::UnknownRevision {
            subject: subject.to_owned(),
            artifact: artifact.to_owned(),
            revision,
            latest,
        },
        _ => unknown_artifact(store, subject, run, artifact).await?,
    })
}

async fn unknown_artifact(
    store: &Store,
    subject: &str,
    run: &str,
    artifact: &str,
) -> Result<Refusal, Error> {
    Ok(Refusal::UnknownArtifact {
        subject: subject.to_owned(),
        artifact: artifact.to_owned(),
        known: store
            .artifact_summaries(run)
            .await?
            .into_iter()
            .map(|row| row.name)
            .collect(),
    })
}

pub(super) fn content(
    subject: &str,
    request: ShowRequest<'_>,
    revision: u32,
    mut files: Vec<(String, Vec<u8>)>,
) -> Result<Vec<u8>, Refusal> {
    let Some(path) = request.file else {
        return Ok(primary(files));
    };
    match files.iter().position(|(name, _)| name == path) {
        Some(index) => Ok(files.swap_remove(index).1),
        None => Err(Refusal::UnknownFile {
            subject: subject.to_owned(),
            artifact: request.artifact.to_owned(),
            revision,
            path: path.to_owned(),
            files: files.into_iter().map(|(name, _)| name).collect(),
        }),
    }
}

fn primary(mut files: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    if files.is_empty() {
        return Vec::new();
    }
    let index = files
        .iter()
        .position(|(_, bytes)| matches!(serde_json::from_slice(bytes), Ok(Value::Array(_))))
        .unwrap_or_default();
    files.swap_remove(index).1
}
