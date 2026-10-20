mod graph;

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use super::contract::OutputRule;
use super::files::read;
use super::portable::{KeyFault, first_key_fault};
use super::{DeliveredFile, DeliveryRejection, ValidatedOutput};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ItemListProblem {
    #[error("the index is not a list of items")]
    NotAList,
    #[error("item {} has no key, needs or files of the right shape", index + 1)]
    MalformedItem { index: usize },
    #[error("key {key} is not a portable item key")]
    InvalidKey { key: String },
    #[error("key {key} is a reserved device name")]
    ReservedKey { key: String },
    #[error("key {key} is used by more than one item")]
    DuplicateKey { key: String },
    #[error("item {key} needs {need}, which is not an item")]
    UnknownNeed { key: String, need: String },
    #[error("needs form a cycle: {}", path.join(" → "))]
    Cycle { path: Vec<String> },
    #[error("item {key} has no {name} file")]
    MissingItemFile { key: String, name: String },
    #[error("item {key} names an undeclared {name} file")]
    UndeclaredItemFile { key: String, name: String },
    #[error("{path} is named by more than one item")]
    DuplicatePath { path: String },
    #[error("{path} is not referenced by any item")]
    UnreferencedFile { path: String },
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Item {
    pub(crate) key: String,
    pub(crate) needs: Vec<String>,
    #[serde(default)]
    pub(crate) files: BTreeMap<String, String>,
}

impl OutputRule {
    pub(super) fn validate_item_list(
        &self,
        delivery_dir: &Path,
        index: DeliveredFile,
        value: Value,
    ) -> Result<ValidatedOutput, DeliveryRejection> {
        let invalid = |problem| DeliveryRejection::InvalidItemList {
            path: index.path.clone(),
            problem,
        };
        let items = parse(&value).map_err(invalid)?;
        check_keys(&items).map_err(invalid)?;
        graph::check_needs(&items).map_err(invalid)?;
        let paths = self.item_paths(&items).map_err(invalid)?;
        check_unique_paths(&index.path, &paths).map_err(invalid)?;
        let mut files = vec![index];
        for (name, path) in paths {
            let file = read(delivery_dir, path)?;
            self.item_files[name].check(&file)?;
            files.push(file);
        }
        Ok(ValidatedOutput {
            files,
            json: Some(value),
        })
    }

    fn item_paths<'a>(
        &self,
        items: &'a [Item],
    ) -> Result<Vec<(&'a str, &'a str)>, ItemListProblem> {
        let mut paths = Vec::new();
        for item in items {
            let problem = |name: &str| (item.key.clone(), name.to_owned());
            if let Some(name) = self
                .item_files
                .keys()
                .find(|name| !item.files.contains_key(*name))
            {
                let (key, name) = problem(name);
                return Err(ItemListProblem::MissingItemFile { key, name });
            }
            if let Some(name) = item
                .files
                .keys()
                .find(|name| !self.item_files.contains_key(*name))
            {
                let (key, name) = problem(name);
                return Err(ItemListProblem::UndeclaredItemFile { key, name });
            }
            paths.extend(
                item.files
                    .iter()
                    .map(|(name, path)| (name.as_str(), path.as_str())),
            );
        }
        Ok(paths)
    }
}

fn parse(value: &Value) -> Result<Vec<Item>, ItemListProblem> {
    let elements = value.as_array().ok_or(ItemListProblem::NotAList)?;
    elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            Item::deserialize(element).map_err(|_| ItemListProblem::MalformedItem { index })
        })
        .collect()
}

fn check_keys(items: &[Item]) -> Result<(), ItemListProblem> {
    let Some((key, fault)) = first_key_fault(items.iter().map(|item| item.key.as_str())) else {
        return Ok(());
    };
    let key = key.to_owned();
    Err(match fault {
        KeyFault::NotPortable => ItemListProblem::InvalidKey { key },
        KeyFault::Reserved => ItemListProblem::ReservedKey { key },
        KeyFault::Duplicate => ItemListProblem::DuplicateKey { key },
    })
}

fn check_unique_paths(index: &str, paths: &[(&str, &str)]) -> Result<(), ItemListProblem> {
    let mut seen = HashSet::from([index.to_ascii_lowercase()]);
    match paths
        .iter()
        .find(|(_, path)| !seen.insert(path.to_ascii_lowercase()))
    {
        Some((_, path)) => Err(ItemListProblem::DuplicatePath {
            path: (*path).to_owned(),
        }),
        None => Ok(()),
    }
}
