use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use jsonschema::{Draft, Retrieve, Uri};
use serde_json::{Value, json};

use super::fixtures::{ACTOR, Feature, agent};
use super::planned::{PLANNER, plan};
use super::repository::git;
use crate::project::{
    BASE_SCHEMA, CHECK_REPORT_SCHEMA, CONFLICT_SCHEMA, Catalog, Config, DeliveryCheck,
    DriveOutcome, INTEGRATION_SCHEMA,
};
use crate::runtime::{
    AgentRuntime, Behavior, Delivery, FakeRuntime, RuntimeCompletion, RuntimeError,
    RuntimeEventSink, RuntimeRequest, Script, StopSignal,
};

const BUILT_WORKFLOW: &str = r#"
schema = 1
name = "built"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.plan]
kind = "agent"
agent = "planner"
needs = ["prepare"]

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

const SHIPPED_WORKFLOW: &str = r#"
schema = 1
name = "shipped"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.plan]
kind = "agent"
agent = "planner"
needs = ["prepare"]

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

[steps.snapshot]
kind = "operation"
run = "git.snapshot_run_branch"
needs = ["implement"]

[steps.ship]
kind = "agent"
agent = "shipper"
needs = ["snapshot"]
"#;

pub(super) const REVIEWED_TASK: &str = r#"
schema = 1
name = "task"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"

[steps.build]
kind = "agent"
agent = "builder"

[steps.check]
kind = "operation"
run = "checks.run"
needs = ["build"]

[steps.review]
kind = "agent"
agent = "reviewer"
needs = ["check"]
when = { step = "check", outcome = "passed" }

[steps.cycle]
kind = "loop"
body = ["build", "check", "review"]
until = { step = "review", outcome = "approved" }
needs = ["setup"]
"#;

pub(super) const INTEGRATED_TASK: &str = r#"
schema = 1
name = "task"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"

[steps.build]
kind = "agent"
agent = "builder"

[steps.check]
kind = "operation"
run = "checks.run"
needs = ["build"]

[steps.review]
kind = "agent"
agent = "reviewer"
needs = ["check"]
when = { step = "check", outcome = "passed" }

[steps.integrate]
kind = "operation"
run = "git.integrate"
needs = ["review"]
when = { step = "review", outcome = "approved" }

[steps.cycle]
kind = "loop"
body = ["build", "check", "review", "integrate"]
until = { step = "integrate", outcome = "merged" }
needs = ["setup"]
"#;

const BUILDER: &str = r#"
schema = 1
id = "builder"

[inputs.item]
format = "json"

[inputs.request]
format = "text"

[inputs.feedback]
format = "json"
optional = true

[inputs.check-report]
format = "json"
optional = true

[inputs.conflict]
format = "json"
optional = true

[outputs.commit]
format = "json"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const REVIEWER: &str = r#"
schema = 1
id = "reviewer"
outcomes = ["approved", "changes_requested"]

[inputs.item]
format = "json"

[inputs.commit]
format = "json"

[inputs.check-report]
format = "json"

[inputs.feedback]
format = "json"
optional = true

[inputs.conflict]
format = "json"
optional = true

[outputs.feedback]
format = "json"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const SHIPPER: &str = r#"
schema = 1
id = "shipper"

[inputs.plan]
format = "item_list"

[inputs.integration]
format = "json"

[outputs.delivery]
format = "json"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const APPROVED: &str = "approved";

pub(super) type Act = fn(&Path) -> String;
pub(super) type Turn = Box<dyn Fn(&Path) -> Behavior + Send + Sync>;

pub(super) struct Team {
    turns: BTreeMap<(String, u32, u32), Turn>,
    requests: Mutex<Vec<RuntimeRequest>>,
}

impl Team {
    pub(super) fn new() -> Team {
        Team {
            turns: BTreeMap::new(),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn on(mut self, step: &str, iteration: u32, attempt: u32, turn: Turn) -> Team {
        self.turns
            .insert((step.to_owned(), iteration, attempt), turn);
        self
    }

    pub(super) fn requests(&self) -> Vec<RuntimeRequest> {
        self.requests.lock().expect("the requests").clone()
    }

    pub(super) fn request(&self, step: &str, iteration: u32) -> RuntimeRequest {
        self.requests()
            .into_iter()
            .rev()
            .find(|request| request.step.step == step && request.step.iteration == iteration)
            .unwrap_or_else(|| panic!("{step} ran in iteration {iteration}"))
    }
}

impl AgentRuntime for Team {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        self.requests
            .lock()
            .expect("the requests")
            .push(request.clone());
        let step = &request.step;
        let key = (step.step.clone(), step.iteration, step.attempt);
        let script = match self.turns.get(&key) {
            Some(turn) => Script::new().on(
                &step.step,
                step.iteration,
                step.attempt,
                turn(&request.launch.working_dir),
            ),
            None => Script::new(),
        };
        FakeRuntime::new(script)
            .execute(request, events, stop)
            .await
    }
}

pub(super) fn builds(act: Act) -> Turn {
    Box::new(move |worktree| {
        let commit = json!({ "sha": act(worktree) }).to_string();
        Behavior::Deliver(Delivery::new("done").output("commit", "commit.json", commit))
    })
}

pub(super) fn reviews(outcome: &'static str) -> Turn {
    reviews_with(outcome, Vec::new())
}

pub(super) fn reviews_with(outcome: &'static str, findings: Vec<Value>) -> Turn {
    Box::new(move |worktree| {
        let head = git(worktree, &["rev-parse", "HEAD"]);
        verdict(outcome, &head, &findings)
    })
}

pub(super) fn reviews_commit(outcome: &'static str, sha: String) -> Turn {
    Box::new(move |_| verdict(outcome, &sha, &[]))
}

pub(super) fn blocking(path: &str) -> Value {
    json!({ "blocking": true, "path": path, "message": "fix this" })
}

fn verdict(outcome: &str, reviewed: &str, findings: &[Value]) -> Behavior {
    let feedback = json!({
        "reviewed_commit": reviewed,
        "summary": format!("{outcome} after reading the code"),
        "findings": findings,
    });
    Behavior::Deliver(Delivery::new(outcome).output(
        "feedback",
        "feedback.json",
        feedback.to_string(),
    ))
}

pub(super) fn ships() -> Turn {
    Box::new(|_| {
        let delivery = json!({ "title": "Add login", "summary": "Adds the login." });
        Behavior::Deliver(Delivery::new("done").output(
            "delivery",
            "delivery.json",
            delivery.to_string(),
        ))
    })
}

pub(super) fn input(request: &RuntimeRequest, name: &str, file: &str) -> Option<Value> {
    let prefix = format!("- {name} rev ");
    let line = request
        .context
        .lines()
        .find(|line| line.starts_with(&prefix))?;
    let root = PathBuf::from(line.rsplit("   ").next()?);
    let bytes = fs::read(root.join(file)).expect("the input file");
    Some(serde_json::from_slice(&bytes).expect("the input is JSON"))
}

pub(super) fn input_line(request: &RuntimeRequest, name: &str) -> Option<String> {
    let prefix = format!("- {name} rev ");
    request
        .context
        .lines()
        .find(|line| line.starts_with(&prefix))
        .and_then(|line| line.split("   ").take(2).last())
        .map(|label| format!("{name} {label}"))
}

struct EngineSchemas;

impl Retrieve for EngineSchemas {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let name = uri.as_str().rsplit('/').next().unwrap_or_default();
        let (_, text) = [
            CHECK_REPORT_SCHEMA,
            BASE_SCHEMA,
            CONFLICT_SCHEMA,
            INTEGRATION_SCHEMA,
        ]
        .into_iter()
        .find(|(path, _)| path.ends_with(name))
        .ok_or_else(|| format!("no engine schema {name}"))?;
        Ok(serde_json::from_str(text)?)
    }
}

pub(super) fn assert_valid(schema: (&str, &str), instance: &Value) {
    let (path, text) = schema;
    let schema: Value = serde_json::from_str(text).expect("the schema is JSON");
    let validator = jsonschema::options()
        .with_draft(Draft::Draft7)
        .with_base_uri(format!("https://surt.invalid/{path}"))
        .with_retriever(EngineSchemas)
        .build(&schema)
        .expect("the schema compiles");
    assert!(validator.is_valid(instance), "{path}: {instance}");
}

fn team_catalog(task: &'static str) -> Catalog {
    Catalog::new()
        .workflow("built", BUILT_WORKFLOW)
        .workflow("shipped", SHIPPED_WORKFLOW)
        .workflow("task", task)
        .agent("planner", agent(PLANNER))
        .agent("builder", agent(BUILDER))
        .agent("reviewer", agent(REVIEWER))
        .agent("shipper", agent(SHIPPER))
        .delivery_check("builder", "commit", DeliveryCheck::CommitEvidence)
        .delivery_check("reviewer", "feedback", DeliveryCheck::ReviewedCommit)
        .delivery_check(
            "reviewer",
            "feedback",
            DeliveryCheck::NoBlockingFinding { outcome: APPROVED },
        )
}

pub(super) async fn built(
    task: &'static str,
    items: &[(&str, &[&str])],
    config: Config,
) -> Feature {
    planned(team_catalog(task), "built", items, config).await
}

pub(super) async fn shipped(items: &[(&str, &[&str])], config: Config) -> Feature {
    planned(team_catalog(INTEGRATED_TASK), "shipped", items, config).await
}

async fn planned(
    catalog: Catalog,
    workflow: &str,
    items: &[(&str, &[&str])],
    config: Config,
) -> Feature {
    let feature = Feature::on_repository(catalog, workflow, config).await;
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

pub(super) async fn failures(feature: &Feature, step: &str) -> Vec<(u32, u32, String, String)> {
    let sql = format!(
        "SELECT s.iteration, e.attempt, e.error_code, e.error_message \
         FROM execution e JOIN step_run s ON s.id = e.step_run_id \
         WHERE s.step_key = '{step}' AND e.state = 'failed' ORDER BY s.iteration, e.attempt"
    );
    feature.dir.rows(&sql).await
}
