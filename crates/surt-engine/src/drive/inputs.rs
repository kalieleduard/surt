use super::driver::Driver;
use super::expand::per_item;
use super::frozen::{CorruptSnapshot, FrozenAgent};
use super::load::Loaded;
use crate::artifacts::item_view;
use crate::runtime::AgentRuntime;
use crate::store::{InputRequest, StepRow, StoredRevision};
use crate::workflow::{AgentStep, PerItemStep};

const ITEM: &str = "item";

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) fn input_requests(
        &self,
        loaded: &Loaded,
        row: &StepRow,
        spec: &AgentStep,
        agent: &FrozenAgent,
    ) -> Result<Vec<InputRequest>, CorruptSnapshot> {
        let parent = self.per_item_parent(loaded, row)?;
        Ok(agent
            .contract
            .inputs
            .keys()
            .map(|input| {
                let artifact = spec.inputs.get(input).unwrap_or(input);
                let own = InputRequest {
                    input: input.clone(),
                    artifact: artifact.clone(),
                    run: row.run.clone(),
                    item: None,
                };
                match (parent, &row.item) {
                    (Some((parent, per_item)), Some(item)) if artifact == ITEM => InputRequest {
                        artifact: per_item.items.clone(),
                        run: parent.run.clone(),
                        item: Some(item.clone()),
                        ..own
                    },
                    (Some((parent, per_item)), _) if per_item.pass.contains(artifact) => {
                        InputRequest {
                            run: parent.run.clone(),
                            ..own
                        }
                    }
                    _ => own,
                }
            })
            .collect())
    }

    fn per_item_parent<'s>(
        &'s self,
        loaded: &'s Loaded,
        row: &StepRow,
    ) -> Result<Option<(&'s StepRow, &'s PerItemStep)>, CorruptSnapshot> {
        let Some(parent) = loaded.parent_of(&row.run) else {
            return Ok(None);
        };
        let spec = self
            .definition(loaded, parent)?
            .step(&parent.key)
            .map(|step| &step.spec);
        Ok(per_item(spec).map(|spec| (parent, spec)))
    }
}

pub(super) fn scoped(
    name: &str,
    row: &StepRow,
    revision: StoredRevision,
) -> Result<StoredRevision, CorruptSnapshot> {
    let Some(item) = row.item.as_deref().filter(|_| name == ITEM) else {
        return Ok(revision);
    };
    let files = item_view(&revision.files, item)
        .ok_or_else(|| CorruptSnapshot::new(item, "the item is missing from its Item List"))?;
    Ok(StoredRevision { files, ..revision })
}
