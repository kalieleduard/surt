use serde_json::Value;
use surt_engine::project::{ArtifactSummary, RevisionApproval, Shown, ShownFile};

use super::origin;
use crate::json::{Fields, base64};

pub(super) fn revision(shown: &Shown) -> Fields {
    Fields::new()
        .with("artifact", shown.artifact.as_str())
        .with("revision", shown.revision)
        .with("origin", origin(shown.origin))
        .with("sha256", shown.sha256.as_str())
        .maybe("approval", shown.approval.as_ref().map(approval))
        .with("files", shown.files.iter().map(file).collect::<Vec<_>>())
        .list("edited", shown.edited.clone())
}

pub(super) fn index(artifacts: &[ArtifactSummary]) -> Fields {
    Fields::new().with(
        "artifacts",
        artifacts.iter().map(artifact).collect::<Vec<_>>(),
    )
}

fn approval(approval: &RevisionApproval) -> Fields {
    let decision = if approval.approved {
        "approve"
    } else {
        "reject"
    };
    Fields::new()
        .with("decision", decision)
        .with("actor", approval.actor.as_str())
        .time("decided_at", Some(approval.decided_at))
}

fn file(file: &ShownFile) -> Value {
    let fields = Fields::new().with("path", file.path.as_str());
    match std::str::from_utf8(&file.content) {
        Ok(text) => fields.with("content", text),
        Err(_) => fields.with("content_base64", base64(&file.content)),
    }
    .into()
}

fn artifact(artifact: &ArtifactSummary) -> Value {
    Fields::new()
        .with("name", artifact.name.as_str())
        .with("revisions", artifact.revisions)
        .with("latest", artifact.latest)
        .with("origin", origin(artifact.origin))
        .with("approved", artifact.approved)
        .with("files", artifact.files)
        .into()
}
