use serde::Serialize;
use serde_json::{Map, Value};
use surt_engine::project::{
    DefinedStep, DefinedStepKind, FrozenDefinition, StepCondition, Timeline, TimelineEvent,
};

#[derive(Serialize)]
pub struct RunDetail {
    definition: DefinitionView,
    events: Vec<EventView>,
}

#[derive(Serialize)]
struct DefinitionView {
    name: String,
    source: String,
    steps: Vec<StepView>,
}

#[derive(Serialize)]
struct StepView {
    key: String,
    needs: Vec<String>,
    when: Option<ConditionView>,
    #[serde(flatten)]
    kind: KindView,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum KindView {
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
        operation: String,
    },
    Loop {
        body: Vec<String>,
        until: ConditionView,
    },
    PerItem {
        items: String,
        workflow: String,
    },
}

#[derive(Serialize)]
struct ConditionView {
    step: String,
    outcome: String,
}

#[derive(Serialize)]
struct EventView {
    at: i64,
    kind: String,
    item: Option<String>,
    step: Option<String>,
    iteration: Option<u32>,
    attempt: Option<u32>,
    execution: Option<String>,
    data: Map<String, Value>,
}

impl RunDetail {
    pub fn of(definition: FrozenDefinition, timeline: Timeline) -> RunDetail {
        RunDetail {
            definition: DefinitionView {
                name: definition.name,
                source: definition.source,
                steps: definition.steps.into_iter().map(StepView::from).collect(),
            },
            events: timeline.events.into_iter().map(EventView::from).collect(),
        }
    }
}

impl From<DefinedStep> for StepView {
    fn from(step: DefinedStep) -> StepView {
        StepView {
            key: step.key,
            needs: step.needs,
            when: step.when.map(ConditionView::from),
            kind: KindView::from(step.kind),
        }
    }
}

impl From<DefinedStepKind> for KindView {
    fn from(kind: DefinedStepKind) -> KindView {
        match kind {
            DefinedStepKind::Agent { agent } => KindView::Agent { agent },
            DefinedStepKind::Approval { artifact } => KindView::Approval { artifact },
            DefinedStepKind::Input { artifact } => KindView::Input { artifact },
            DefinedStepKind::Operation { run } => KindView::Operation { operation: run },
            DefinedStepKind::Loop { body, until } => KindView::Loop {
                body,
                until: ConditionView::from(until),
            },
            DefinedStepKind::PerItem { items, workflow } => KindView::PerItem { items, workflow },
        }
    }
}

impl From<StepCondition> for ConditionView {
    fn from(condition: StepCondition) -> ConditionView {
        ConditionView {
            step: condition.step,
            outcome: condition.outcome,
        }
    }
}

impl From<TimelineEvent> for EventView {
    fn from(event: TimelineEvent) -> EventView {
        EventView {
            at: event.at,
            kind: event.kind,
            item: event.item,
            step: event.step,
            iteration: event.iteration,
            attempt: event.attempt,
            execution: event.execution,
            data: event.data,
        }
    }
}
