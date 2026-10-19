use std::collections::BTreeSet;

use super::{Available, produced};
use crate::workflow::agent::Format;
use crate::workflow::definition::Step;
use crate::workflow::error::Problem;
use crate::workflow::inputs::RunInputs;
use crate::workflow::steps::PerItemStep;

const ITEM: &str = "item";

pub(in crate::workflow) struct ChildUse {
    pub(in crate::workflow) step: String,
    pub(in crate::workflow) workflow: String,
    pub(in crate::workflow) external: External,
}

pub(in crate::workflow) struct External {
    pub(in crate::workflow) artifacts: Available,
    pub(in crate::workflow) git_refs: BTreeSet<String>,
}

impl External {
    pub(in crate::workflow) fn of_inputs(inputs: &RunInputs) -> External {
        External {
            artifacts: inputs
                .declared
                .iter()
                .map(|(name, spec)| (name.to_owned(), Some(spec.format.artifact_format())))
                .collect(),
            git_refs: inputs.git_refs().map(str::to_owned).collect(),
        }
    }
}

pub(super) fn child_use(
    step: &Step,
    spec: &PerItemStep,
    parent: &External,
    available: &Available,
) -> Result<ChildUse, Problem> {
    if produced(&spec.items, available)? != Some(Format::ItemList) {
        return Err(Problem::NotAnItemList {
            artifact: spec.items.clone(),
        });
    }
    let mut external = External {
        artifacts: Available::from([(ITEM.to_owned(), Some(Format::Json))]),
        git_refs: BTreeSet::new(),
    };
    for name in &spec.pass {
        external
            .artifacts
            .insert(name.clone(), produced(name, available)?);
        if parent.git_refs.contains(name) {
            external.git_refs.insert(name.clone());
        }
    }
    Ok(ChildUse {
        step: step.key.clone(),
        workflow: spec.workflow.clone(),
        external,
    })
}
