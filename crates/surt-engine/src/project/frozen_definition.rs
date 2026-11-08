use super::show::item_run;
use super::{Error, InspectRequest, Project};
use crate::drive::CorruptSnapshot;
use crate::store::{FrozenFile, Store};
use crate::workflow::{Condition, Definition, StepSpec, WORKFLOW_FILE};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenDefinition {
    pub name: String,
    pub source: String,
    pub steps: Vec<DefinedStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinedStep {
    pub key: String,
    pub needs: Vec<String>,
    pub when: Option<StepCondition>,
    pub kind: DefinedStepKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefinedStepKind {
    Agent {
        agent: String,
    },
    Approval {
        artifact: String,
    },
    Input {
        artifact: String,
    },
    Operation {
        run: String,
    },
    Loop {
        body: Vec<String>,
        until: StepCondition,
    },
    PerItem {
        items: String,
        workflow: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepCondition {
    pub step: String,
    pub outcome: String,
}

impl Project {
    pub async fn frozen_definition(
        &self,
        reference: &str,
        request: InspectRequest<'_>,
    ) -> Result<FrozenDefinition, Error> {
        let subject = self.resolve_subject(reference).await?.name;
        let store = self.reader().await?;
        let definition = self.read_frozen_definition(&store, &subject, request).await;
        store.close().await;
        definition
    }

    async fn read_frozen_definition(
        &self,
        store: &Store,
        subject: &str,
        request: InspectRequest<'_>,
    ) -> Result<FrozenDefinition, Error> {
        let root = self.select_run(store, subject, request.run).await?;
        let run = match request.item {
            None => root.id,
            Some(item) => item_run(store, subject, &root.id, item).await?,
        };
        let snapshot = store.run_definition(&run).await?;
        let file = store
            .frozen_file(&snapshot, WORKFLOW_FILE)
            .await?
            .ok_or_else(|| CorruptSnapshot::new(&snapshot, "no such workflow"))?;
        Ok(described(file)?)
    }
}

fn described(file: FrozenFile) -> Result<FrozenDefinition, CorruptSnapshot> {
    let source = String::from_utf8(file.bytes)
        .map_err(|_| CorruptSnapshot::new(&file.name, format!("{WORKFLOW_FILE} is not UTF-8")))?;
    let definition = Definition::parse(&file.name, &source)
        .map_err(|invalid| CorruptSnapshot::new(&file.name, format!("{invalid:?}")))?;
    let steps = definition
        .steps
        .iter()
        .map(|step| DefinedStep::new(&step.key, &step.spec))
        .collect();
    Ok(FrozenDefinition {
        name: definition.name,
        source,
        steps,
    })
}

impl DefinedStep {
    fn new(key: &str, spec: &StepSpec) -> DefinedStep {
        DefinedStep {
            key: key.to_owned(),
            needs: spec.needs().to_vec(),
            when: spec.when().map(StepCondition::from),
            kind: DefinedStepKind::from(spec),
        }
    }
}

impl From<&StepSpec> for DefinedStepKind {
    fn from(spec: &StepSpec) -> DefinedStepKind {
        match spec {
            StepSpec::Agent(step) => DefinedStepKind::Agent {
                agent: step.agent.clone(),
            },
            StepSpec::Approval(step) => DefinedStepKind::Approval {
                artifact: step.artifact.clone(),
            },
            StepSpec::Input(step) => DefinedStepKind::Input {
                artifact: step.artifact.clone(),
            },
            StepSpec::Operation(step) => DefinedStepKind::Operation {
                run: step.run.clone(),
            },
            StepSpec::Loop(step) => DefinedStepKind::Loop {
                body: step.body.clone(),
                until: StepCondition::from(&step.until),
            },
            StepSpec::PerItem(step) => DefinedStepKind::PerItem {
                items: step.items.clone(),
                workflow: step.workflow.clone(),
            },
        }
    }
}

impl From<&Condition> for StepCondition {
    fn from(condition: &Condition) -> StepCondition {
        StepCondition {
            step: condition.step.clone(),
            outcome: condition.outcome.clone(),
        }
    }
}
