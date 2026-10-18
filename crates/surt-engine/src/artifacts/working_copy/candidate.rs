use std::collections::BTreeSet;
use std::path::Path;

use super::disk::Inspection;
use crate::artifacts::{DeliveryRejection, ItemListProblem, OutputRule};
use crate::store::BaselineFile;

pub(super) struct Candidate<'a> {
    pub(super) dir: &'a Path,
    pub(super) index: &'a str,
    pub(super) baseline: &'a [BaselineFile],
    pub(super) inspection: &'a Inspection,
}

impl Candidate<'_> {
    pub(super) fn validate(
        &self,
        rule: &OutputRule,
    ) -> Result<Vec<(String, Vec<u8>)>, DeliveryRejection> {
        let validated = rule.validate(self.dir, self.index)?;
        let referenced: BTreeSet<&str> = validated
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        if let Some(path) = self.present().find(|path| !referenced.contains(path)) {
            return Err(DeliveryRejection::InvalidItemList {
                path: self.index.to_owned(),
                problem: ItemListProblem::UnreferencedFile {
                    path: path.to_owned(),
                },
            });
        }
        Ok(validated
            .files
            .into_iter()
            .map(|file| (file.path, file.bytes))
            .collect())
    }

    fn present(&self) -> impl Iterator<Item = &str> {
        self.baseline
            .iter()
            .map(|file| file.rel_path.as_str())
            .filter(|path| {
                !self
                    .inspection
                    .missing
                    .iter()
                    .any(|missing| missing == path)
            })
            .chain(self.inspection.extra.iter().map(String::as_str))
    }
}
