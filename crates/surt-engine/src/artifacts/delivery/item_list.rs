mod graph;

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use super::contract::OutputRule;
use super::files::read;
use super::portable::{is_device_name, is_item_key};
use super::{DeliveredFile, DeliveryRejection, ValidatedOutput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ItemListProblem {
    NotAList,
    MalformedItem { index: usize },
    InvalidKey { key: String },
    ReservedKey { key: String },
    DuplicateKey { key: String },
    UnknownNeed { key: String, need: String },
    Cycle { path: Vec<String> },
    MissingItemFile { key: String, name: String },
    UndeclaredItemFile { key: String, name: String },
    DuplicatePath { path: String },
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
    let mut seen = HashSet::new();
    for item in items {
        let key = || item.key.clone();
        if !is_item_key(&item.key) {
            return Err(ItemListProblem::InvalidKey { key: key() });
        }
        if is_device_name(&item.key) {
            return Err(ItemListProblem::ReservedKey { key: key() });
        }
        if !seen.insert(item.key.to_ascii_lowercase()) {
            return Err(ItemListProblem::DuplicateKey { key: key() });
        }
    }
    Ok(())
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
