use super::driver::{Driver, known};
use super::frozen::CorruptSnapshot;
use super::graph::{StepRunId, StepRunState};
use super::load::{ItemLists, Loaded};
use crate::artifacts::{ListedItem, listed_items};
use crate::project::Error;
use crate::runtime::AgentRuntime;
use crate::store::{Expansion, ListRevision, NewChild, RunRows};
use crate::workflow::{PerItemStep, StepSpec};

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn start_iteration(
        &self,
        loaded: &Loaded,
        loop_: &StepRunId,
    ) -> Result<(), Error> {
        let row = known(loaded, loop_)?;
        let definition = self.definition(loaded, row)?;
        let Some(StepSpec::Loop(spec)) = definition.step(&row.key).map(|step| &step.spec) else {
            return Err(CorruptSnapshot::new(&row.key, "not a loop").into());
        };
        let body: Vec<(&str, &str)> = spec
            .body
            .iter()
            .filter_map(|key| definition.step(key))
            .map(|step| (step.key.as_str(), step.spec.kind().as_str()))
            .collect();
        Ok(self.driving.store.start_iteration(&row.id, &body).await?)
    }

    pub(super) async fn item_lists(&self, rows: &RunRows) -> Result<ItemLists, Error> {
        let mut lists = ItemLists::new();
        for row in &rows.steps {
            if row.state == StepRunState::Pending {
                continue;
            }
            let definition = self.frozen.definition(rows.definition_of(&row.run))?;
            let Some(spec) = per_item(definition.step(&row.key).map(|step| &step.spec)) else {
                continue;
            };
            let revision = self
                .driving
                .store
                .expanded_revision(&row.run, &spec.items)
                .await?;
            lists.insert(row.id.clone(), items(&spec.items, revision)?.1);
        }
        Ok(lists)
    }

    pub(super) async fn expand(&self, loaded: &Loaded, step: &StepRunId) -> Result<(), Error> {
        let row = known(loaded, step)?;
        let spec = per_item(
            self.definition(loaded, row)?
                .step(&row.key)
                .map(|step| &step.spec),
        )
        .ok_or_else(|| CorruptSnapshot::new(&row.key, "not a per_item step"))?;
        let revision = self
            .driving
            .store
            .latest_revision(&row.run, &spec.items)
            .await?;
        let (revision, listed) = items(&spec.items, revision)?;
        let (snapshot, definition) = self.frozen.definition_named(&spec.workflow)?;
        let steps: Vec<(&str, &str)> = definition
            .steps
            .iter()
            .filter(|step| definition.enclosing_loop(&step.key).is_none())
            .map(|step| (step.key.as_str(), step.spec.kind().as_str()))
            .collect();
        let expansion = Expansion {
            step: &row.id,
            revision: &revision,
            children: listed
                .iter()
                .map(|item| NewChild {
                    item: &item.key,
                    definition: snapshot,
                    steps: steps.clone(),
                })
                .collect(),
        };
        Ok(self.driving.store.expand_items(expansion).await?)
    }
}

pub(super) fn per_item(spec: Option<&StepSpec>) -> Option<&PerItemStep> {
    match spec {
        Some(StepSpec::PerItem(spec)) => Some(spec),
        _ => None,
    }
}

fn items(
    artifact: &str,
    revision: Option<ListRevision>,
) -> Result<(String, Vec<ListedItem>), CorruptSnapshot> {
    let revision =
        revision.ok_or_else(|| CorruptSnapshot::new(artifact, "the Item List has no Revision"))?;
    let listed = listed_items(&revision.files)
        .ok_or_else(|| CorruptSnapshot::new(artifact, "the Item List index is unreadable"))?;
    Ok((revision.id, listed))
}
