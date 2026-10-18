use serde_json::{Value, json};

use super::WorkingCopy;
use crate::project::{Error, Refusal};
use crate::store::Store;

const EDIT_REFUSED: &str = "working_copy.edit_refused";

impl WorkingCopy {
    pub(super) async fn refused(&self, store: &Store, error: Error) -> Error {
        let Some(data) = details(&error) else {
            return error;
        };
        match store
            .record_run_event(&self.root, EDIT_REFUSED, &data)
            .await
        {
            Ok(()) => error,
            Err(recording) => recording.into(),
        }
    }
}

fn details(error: &Error) -> Option<Value> {
    let Error::Refused(refusal) = error else {
        return None;
    };
    match refusal {
        Refusal::PinnedEdit {
            artifact,
            revision,
            files,
            ..
        } => Some(json!({
            "artifact": artifact,
            "reason": "pinned",
            "revision": revision,
            "files": files,
        })),
        Refusal::InvalidEdit {
            artifact,
            problem,
            files,
            ..
        } => Some(json!({
            "artifact": artifact,
            "reason": "invalid",
            "problem": problem,
            "files": files,
        })),
        _ => None,
    }
}
