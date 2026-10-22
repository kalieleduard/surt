use jiff::civil::Date;

use super::naming::{DATE_FORMAT, Slug, dated_name, first_free, next_numbered};
use super::{Error, Project, open_store};
use crate::drive::Frozen;
use crate::store::Store;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreeName {
    pub name: String,
    pub stem: String,
}

impl FreeName {
    pub fn is_suffixed(&self) -> bool {
        self.name != self.stem
    }
}

impl Project {
    pub async fn free_subject_name(&self, slug: &Slug, date: Date) -> Result<FreeName, Error> {
        let stem = dated_name(slug, date);
        let store = open_store(&self.layout).await?;
        let taken = store.subject_names_starting_with(&stem).await;
        store.close().await;
        Ok(FreeName {
            name: first_free(&stem, &taken?),
            stem,
        })
    }

    pub async fn next_numbered_subject_name(&self, date: Date) -> Result<FreeName, Error> {
        let store = open_store(&self.layout).await?;
        let taken = store
            .subject_names_starting_with(&format!("{}-", date.strftime(DATE_FORMAT)))
            .await;
        store.close().await;
        let name = next_numbered(date, &taken?);
        Ok(FreeName {
            stem: name.clone(),
            name,
        })
    }

    pub async fn latest_workflow(&self, subject: &str) -> Result<Option<String>, Error> {
        let store = open_store(&self.layout).await?;
        let workflow = latest_workflow(&store, subject).await;
        store.close().await;
        workflow
    }
}

async fn latest_workflow(store: &Store, subject: &str) -> Result<Option<String>, Error> {
    let runs = store.root_runs(subject).await?;
    let Some(latest) = runs.last() else {
        return Ok(None);
    };
    let frozen = Frozen::from_files(store.frozen_files(&latest.id).await?)?;
    Ok(Some(frozen.definition(&latest.definition)?.name.clone()))
}
