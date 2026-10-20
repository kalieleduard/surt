use std::collections::BTreeMap;

use super::frozen::{CorruptSnapshot, Frozen};
use super::graph::{
    Attempts, Condition, Item, ItemKey, Iterations, OperationId, Outcome, RunGraph, RunId,
    RunState, Step, StepKey, StepRunId, Work, WorkflowRun,
};
use super::outcome::StepLabel;
use crate::artifacts::ListedItem;
use crate::store::{RunRows, StepRow};
use crate::workflow::{Definition, StepSpec};

pub(crate) type ItemLists = BTreeMap<String, Vec<ListedItem>>;

pub(crate) struct Loaded {
    pub(crate) graph: RunGraph,
    pub(crate) rows: RunRows,
}

struct Builder<'a> {
    frozen: &'a Frozen,
    rows: &'a RunRows,
    lists: &'a ItemLists,
}

impl Loaded {
    pub(crate) fn new(
        frozen: &Frozen,
        rows: RunRows,
        lists: &ItemLists,
    ) -> Result<Loaded, CorruptSnapshot> {
        let builder = Builder {
            frozen,
            rows: &rows,
            lists,
        };
        let root = builder.run(&rows.id, rows.state, &rows.definition)?;
        let graph = RunGraph {
            root,
            cancel_requested: rows.cancel_requested,
        };
        Ok(Loaded { graph, rows })
    }

    pub(crate) fn step(&self, id: &StepRunId) -> Option<&StepRow> {
        self.rows.steps.iter().find(|row| row.id == id.0)
    }

    pub(crate) fn definition_of(&self, run: &str) -> &str {
        self.rows.definition_of(run)
    }

    pub(crate) fn parent_of(&self, run: &str) -> Option<&StepRow> {
        let child = self.rows.children.iter().find(|child| child.id == run)?;
        self.rows
            .steps
            .iter()
            .find(|row| row.id == child.parent_step)
    }

    pub(crate) fn previous_iteration(&self, row: &StepRow) -> Vec<&StepRow> {
        let previous = row.iteration.map(|iteration| iteration.saturating_sub(1));
        self.rows
            .steps
            .iter()
            .filter(|other| {
                other.loop_step.is_some()
                    && other.loop_step == row.loop_step
                    && other.iteration == previous
            })
            .collect()
    }
}

pub(crate) fn label(row: &StepRow) -> StepLabel {
    StepLabel {
        key: row.key.clone(),
        item: row.item.clone(),
        iteration: row.iteration,
    }
}

impl Builder<'_> {
    fn run(
        &self,
        id: &str,
        state: RunState,
        snapshot: &str,
    ) -> Result<WorkflowRun, CorruptSnapshot> {
        let definition = self.frozen.definition(snapshot)?;
        let mut steps = Vec::new();
        for step in &definition.steps {
            if definition.enclosing_loop(&step.key).is_some() {
                continue;
            }
            let row = self
                .rows
                .steps
                .iter()
                .find(|row| row.run == id && row.loop_step.is_none() && row.key == step.key);
            if let Some(row) = row {
                steps.push(self.step(definition, row, &step.spec)?);
            }
        }
        Ok(WorkflowRun {
            id: RunId(id.to_owned()),
            state,
            steps,
        })
    }

    fn step(
        &self,
        definition: &Definition,
        row: &StepRow,
        spec: &StepSpec,
    ) -> Result<Step, CorruptSnapshot> {
        Ok(Step {
            id: StepRunId(row.id.clone()),
            key: StepKey(row.key.clone()),
            needs: spec
                .needs()
                .iter()
                .map(|need| StepKey(need.clone()))
                .collect(),
            when: spec.when().map(|when| condition(&when.step, &when.outcome)),
            state: row.state.clone(),
            work: self.work(definition, row, spec)?,
        })
    }

    fn work(
        &self,
        definition: &Definition,
        row: &StepRow,
        spec: &StepSpec,
    ) -> Result<Work, CorruptSnapshot> {
        Ok(match spec {
            StepSpec::Agent(_) => Work::Agent(self.attempts(row)),
            StepSpec::Operation(operation) => {
                Work::Operation(OperationId(operation.run.clone()), self.attempts(row))
            }
            StepSpec::Approval(_) => Work::Approval,
            StepSpec::Input(_) => Work::Input,
            StepSpec::Loop(spec) => Work::Loop(
                condition(&spec.until.step, &spec.until.outcome),
                self.iterations(definition, row, &spec.body)?,
            ),
            StepSpec::PerItem(_) => Work::PerItem(self.items(row)?),
        })
    }

    fn attempts(&self, row: &StepRow) -> Attempts {
        Attempts {
            executions: self
                .rows
                .executions
                .iter()
                .filter(|execution| execution.step == row.id)
                .map(|execution| execution.state)
                .collect(),
            budget_start: row.budget_base,
        }
    }

    fn iterations(
        &self,
        definition: &Definition,
        row: &StepRow,
        body: &[String],
    ) -> Result<Iterations, CorruptSnapshot> {
        let members: Vec<&StepRow> = self
            .rows
            .steps
            .iter()
            .filter(|member| member.loop_step.as_deref() == Some(row.id.as_str()))
            .collect();
        let count = members
            .iter()
            .filter_map(|member| member.iteration)
            .max()
            .unwrap_or_default();
        let mut bodies = Vec::new();
        for iteration in 1..=count {
            let mut steps = Vec::new();
            for key in body {
                let member = members
                    .iter()
                    .find(|member| member.iteration == Some(iteration) && &member.key == key);
                if let (Some(member), Some(step)) = (member, definition.step(key)) {
                    steps.push(self.step(definition, member, &step.spec)?);
                }
            }
            bodies.push(steps);
        }
        Ok(Iterations {
            bodies,
            budget_start: row.budget_base,
        })
    }

    fn items(&self, row: &StepRow) -> Result<Vec<Item>, CorruptSnapshot> {
        let Some(listed) = self.lists.get(&row.id) else {
            return Ok(Vec::new());
        };
        let mut items = Vec::new();
        for item in listed {
            let child = self
                .rows
                .children
                .iter()
                .find(|child| child.parent_step == row.id && child.item == item.key)
                .ok_or_else(|| CorruptSnapshot::new(&item.key, "no child run for the item"))?;
            items.push(Item {
                key: ItemKey(item.key.clone()),
                needs: item
                    .needs
                    .iter()
                    .map(|need| ItemKey(need.clone()))
                    .collect(),
                run: self.run(&child.id, child.state, &child.definition)?,
            });
        }
        Ok(items)
    }
}

fn condition(step: &str, outcome: &str) -> Condition {
    Condition {
        step: StepKey(step.to_owned()),
        outcome: Outcome(outcome.to_owned()),
    }
}
