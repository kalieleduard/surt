use std::fs::{self, Metadata};
use std::path::Path;

use serde_json::Value;

use super::contract::{FileRule, OutputRule};
use super::portable::is_portable_path;
use super::{DeliveredFile, DeliveryRejection, ValidatedOutput};
use crate::platform;
use crate::workflow::Format;

impl OutputRule {
    pub(crate) fn validate(
        &self,
        delivery_dir: &Path,
        path: &str,
    ) -> Result<ValidatedOutput, DeliveryRejection> {
        let file = read(delivery_dir, path)?;
        let json = self.file.check(&file)?;
        if let (Format::ItemList, Some(value)) = (self.file.format, json.clone()) {
            return self.validate_item_list(delivery_dir, file, value);
        }
        Ok(ValidatedOutput {
            files: vec![file],
            json,
        })
    }
}

impl FileRule {
    pub(super) fn check(&self, file: &DeliveredFile) -> Result<Option<Value>, DeliveryRejection> {
        match self.format {
            Format::Text | Format::Markdown => check_text(file).map(|()| None),
            Format::Json | Format::ItemList => self.check_json(file).map(Some),
        }
    }

    fn check_json(&self, file: &DeliveredFile) -> Result<Value, DeliveryRejection> {
        let value: Value = serde_json::from_slice(&file.bytes).map_err(|error| {
            DeliveryRejection::InvalidJson {
                path: file.path.clone(),
                detail: error.to_string(),
            }
        })?;
        match self.schema.as_ref().map(|schema| schema.validate(&value)) {
            Some(Err(error)) => Err(DeliveryRejection::SchemaViolation {
                path: file.path.clone(),
                detail: error.to_string(),
            }),
            _ => Ok(value),
        }
    }
}

pub(super) fn read(delivery_dir: &Path, path: &str) -> Result<DeliveredFile, DeliveryRejection> {
    let owned = || path.to_owned();
    if !is_portable_path(path) {
        return Err(DeliveryRejection::UnsafePath { path: owned() });
    }
    let metadata = link_free_metadata(delivery_dir, path)?;
    if !metadata.is_file() {
        return Err(DeliveryRejection::NotRegularFile { path: owned() });
    }
    let bytes = fs::read(delivery_dir.join(path))
        .map_err(|_| DeliveryRejection::MissingFile { path: owned() })?;
    Ok(DeliveredFile {
        path: owned(),
        bytes,
    })
}

fn link_free_metadata(delivery_dir: &Path, path: &str) -> Result<Metadata, DeliveryRejection> {
    let missing = || DeliveryRejection::MissingFile {
        path: path.to_owned(),
    };
    let mut current = delivery_dir.to_path_buf();
    let mut last = None;
    for component in path.split('/') {
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|_| missing())?;
        if platform::is_link(&metadata) {
            return Err(DeliveryRejection::Link {
                path: path.to_owned(),
            });
        }
        last = Some(metadata);
    }
    last.ok_or_else(missing)
}

fn check_text(file: &DeliveredFile) -> Result<(), DeliveryRejection> {
    let path = || file.path.clone();
    if file.bytes.is_empty() {
        return Err(DeliveryRejection::EmptyFile { path: path() });
    }
    match std::str::from_utf8(&file.bytes) {
        Ok(_) => Ok(()),
        Err(_) => Err(DeliveryRejection::NotUtf8 { path: path() }),
    }
}
