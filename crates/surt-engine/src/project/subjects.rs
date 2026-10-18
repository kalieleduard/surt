use jiff::civil::Date;

use super::naming::{Slug, dated_name, first_free, is_named_after};
use super::{Error, Project, open_store};

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

    pub async fn active_subject_with_slug(&self, slug: &Slug) -> Result<Option<String>, Error> {
        let store = open_store(&self.layout).await?;
        let names = store.names_of_subjects_with_active_runs().await;
        store.close().await;
        Ok(names?.into_iter().find(|name| is_named_after(name, slug)))
    }
}
