use surt_engine::project::RunStatus;

use crate::driving::base_merge::{MergeWording, wording};

pub(super) fn result(status: &RunStatus) -> Option<String> {
    Some(
        match wording(status.merge.as_ref()?, status.base.as_ref())? {
            MergeWording::Merged(text) => text,
            MergeWording::NotMerged(why) => format!("not merged: {why}"),
        },
    )
}
