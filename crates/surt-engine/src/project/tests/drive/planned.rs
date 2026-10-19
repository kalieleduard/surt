use std::sync::Mutex;

use tokio::sync::watch;

use super::super::fixtures::Catalog;
use super::fixtures::{ACTOR, Feature, agent};
use crate::project::{Config, DriveOutcome};
use crate::runtime::{
    AgentRuntime, Behavior, Delivery, FakeRuntime, RuntimeCompletion, RuntimeError,
    RuntimeEventSink, RuntimeRequest, Script, StopSignal,
};

pub(super) const PLANNED_WORKFLOW: &str = r#"
schema = 1
name = "planned"

[steps.plan]
kind = "agent"
agent = "planner"

[steps.approve]
kind = "approval"
artifact = "plan"
rejection_artifact = "plan-rejection"
needs = ["plan"]

[steps.implement]
kind = "per_item"
items = "plan"
workflow = "task"
pass = ["request"]
needs = ["approve"]
"#;

pub(super) const TASK_WORKFLOW: &str = r#"
schema = 1
name = "task"

[steps.build]
kind = "agent"
agent = "builder"
"#;

pub(super) const CYCLE_WORKFLOW: &str = r#"
schema = 1
name = "task"

[steps.build]
kind = "agent"
agent = "builder"

[steps.review]
kind = "agent"
agent = "reviewer"
needs = ["build"]

[steps.cycle]
kind = "loop"
body = ["build", "review"]
until = { step = "review", outcome = "approved" }
"#;

pub(super) const REVIEWER: &str = r#"
schema = 1
id = "reviewer"
outcomes = ["approved", "changes_requested"]

[inputs.item]
format = "json"

[inputs.work]
format = "text"

[outputs.feedback]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

pub(super) const PLANNER: &str = r#"
schema = 1
id = "planner"

[inputs.request]
format = "text"

[outputs.plan]
format = "item_list"

[outputs.plan.item_files.task]
format = "markdown"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

pub(super) const BUILDER: &str = r#"
schema = 1
id = "builder"

[inputs.item]
format = "json"

[inputs.request]
format = "text"

[outputs.work]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

pub(super) fn planned_catalog(task: &'static str) -> Catalog {
    Catalog::new()
        .workflow("planned", PLANNED_WORKFLOW)
        .workflow("task", task)
        .agent("planner", agent(PLANNER))
        .agent("builder", agent(BUILDER))
        .agent("reviewer", agent(REVIEWER))
}

pub(super) fn review(outcome: &str) -> Behavior {
    Behavior::Deliver(Delivery::new(outcome).output("feedback", "feedback.txt", outcome))
}

pub(super) fn plan(items: &[(&str, &[&str])]) -> Behavior {
    let index: Vec<String> = items
        .iter()
        .map(|(key, needs)| {
            let needs: Vec<String> = needs.iter().map(|need| format!("\"{need}\"")).collect();
            format!(
                r#"{{"key": "{key}", "needs": [{}], "title": "Task {key}", "files": {{"task": "tasks/{key}.md"}}}}"#,
                needs.join(", ")
            )
        })
        .collect();
    let delivery = items.iter().fold(
        Delivery::new("done").output("plan", "plan.json", format!("[{}]", index.join(", "))),
        |delivery, (key, _)| delivery.file(&format!("tasks/{key}.md"), format!("# Task {key}\n")),
    );
    Behavior::Deliver(delivery)
}

pub(super) fn work(text: &str) -> Behavior {
    Behavior::Deliver(Delivery::new("done").output("work", "work.txt", text))
}

pub(super) async fn approved(task: &'static str, items: &[(&str, &[&str])]) -> Feature {
    approved_with(task, items, Config::default()).await
}

pub(super) async fn approved_with(
    task: &'static str,
    items: &[(&str, &[&str])],
    config: Config,
) -> Feature {
    let feature = Feature::with_config(planned_catalog(task), "planned", config).await;
    let planned = feature
        .drive(Script::new().on("plan", 1, 1, plan(items)))
        .await
        .outcome
        .expect("the plan reaches its gate");
    assert!(
        matches!(planned, DriveOutcome::AtGate { .. }),
        "{planned:?}"
    );
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the plan is approved");
    feature
}

pub(super) struct Gauge {
    fake: FakeRuntime,
    target: usize,
    active: Mutex<usize>,
    peak: watch::Sender<usize>,
    started: Mutex<Vec<String>>,
}

impl Gauge {
    pub(super) fn new(script: Script, target: usize) -> Gauge {
        Gauge {
            fake: FakeRuntime::new(script),
            target,
            active: Mutex::new(0),
            peak: watch::Sender::new(0),
            started: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn peak(&self) -> usize {
        *self.peak.borrow()
    }

    pub(super) fn started(&self) -> Vec<String> {
        self.started.lock().expect("the started steps").clone()
    }

    fn enter(&self, step: &str) {
        self.started
            .lock()
            .expect("the started steps")
            .push(step.to_owned());
        let mut active = self.active.lock().expect("the active count");
        *active += 1;
        let now = *active;
        self.peak.send_modify(|peak| *peak = (*peak).max(now));
    }

    fn leave(&self) {
        *self.active.lock().expect("the active count") -= 1;
    }
}

impl AgentRuntime for Gauge {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        self.enter(&request.step.step);
        let mut peak = self.peak.subscribe();
        let _ = peak.wait_for(|peak| *peak >= self.target).await;
        let completion = self.fake.execute(request, events, stop).await;
        self.leave();
        completion
    }
}
