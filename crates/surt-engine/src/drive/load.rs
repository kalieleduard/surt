use super::graph::{
    Attempts, Condition, Iterations, OperationId, Outcome, RunGraph, RunId, Step, StepKey,
    StepRunId, Work, WorkflowRun,
};
use super::outcome::StepLabel;
use crate::store::{RunRows, StepRow};
use crate::workflow::{Definition, StepSpec};

pub(crate) struct Loaded {
    pub(crate) graph: RunGraph,
    pub(crate) rows: RunRows,
}

struct Builder<'a> {
    definition: &'a Definition,
    rows: &'a RunRows,
}

impl Loaded {
    pub(crate) fn new(definition: &Definition, rows: RunRows) -> Loaded {
        let builder = Builder {
            definition,
            rows: &rows,
        };
        let steps = definition
            .steps
            .iter()
            .filter(|step| definition.enclosing_loop(&step.key).is_none())
            .filter_map(|step| {
                let row = rows.steps.iter().find(|row| {
                    row.run == rows.id && row.loop_step.is_none() && row.key == step.key
                })?;
                Some(builder.step(row, &step.spec))
            })
            .collect();
        let graph = RunGraph {
            root: WorkflowRun {
                id: RunId(rows.id.clone()),
                state: rows.state,
                steps,
            },
            cancel_requested: rows.cancel_requested,
        };
        Loaded { graph, rows }
    }

    pub(crate) fn step(&self, id: &StepRunId) -> Option<&StepRow> {
        self.rows.steps.iter().find(|row| row.id == id.0)
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
        item: None,
        iteration: row.iteration,
    }
}

impl Builder<'_> {
    fn step(&self, row: &StepRow, spec: &StepSpec) -> Step {
        Step {
            id: StepRunId(row.id.clone()),
            key: StepKey(row.key.clone()),
            needs: spec
                .needs()
                .iter()
                .map(|need| StepKey(need.clone()))
                .collect(),
            when: spec.when().map(|when| condition(&when.step, &when.outcome)),
            state: row.state.clone(),
            work: self.work(row, spec),
        }
    }

    fn work(&self, row: &StepRow, spec: &StepSpec) -> Work {
        match spec {
            StepSpec::Agent(_) => Work::Agent(self.attempts(row)),
            StepSpec::Operation(operation) => {
                Work::Operation(OperationId(operation.run.clone()), self.attempts(row))
            }
            StepSpec::Approval(_) => Work::Approval,
            StepSpec::Loop(spec) => Work::Loop(
                condition(&spec.until.step, &spec.until.outcome),
                self.iterations(row, &spec.body),
            ),
            StepSpec::PerItem(_) => Work::PerItem(Vec::new()),
        }
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
            budget_start: 0,
        }
    }

    fn iterations(&self, row: &StepRow, body: &[String]) -> Iterations {
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
        let bodies = (1..=count)
            .map(|iteration| {
                body.iter()
                    .filter_map(|key| {
                        let member = members.iter().find(|member| {
                            member.iteration == Some(iteration) && &member.key == key
                        })?;
                        let spec = &self.definition.step(key)?.spec;
                        Some(self.step(member, spec))
                    })
                    .collect()
            })
            .collect();
        Iterations {
            bodies,
            budget_start: 0,
        }
    }
}

fn condition(step: &str, outcome: &str) -> Condition {
    Condition {
        step: StepKey(step.to_owned()),
        outcome: Outcome(outcome.to_owned()),
    }
}
