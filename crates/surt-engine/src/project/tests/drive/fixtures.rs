use std::fs;
use std::path::Path;
use std::sync::Mutex;

use tokio::sync::mpsc;

use super::super::fixtures::{ProjectDir, default_config, new_run};
use crate::project::{
    AgentAssets, Catalog, Config, DriveOutcome, Error, Progress, Project, RunId, StepLabel,
};
use crate::runtime::{
    AgentRuntime, Behavior, Delivery, FakeRuntime, RuntimeCompletion, RuntimeError,
    RuntimeEventSink, RuntimeRequest, Script, StopSignal,
};

pub(super) const REVIEW_WORKFLOW: &str = r#"
schema = 1
name = "review"

[steps.write]
kind = "agent"
agent = "writer"

[steps.approve]
kind = "approval"
artifact = "draft"
rejection_artifact = "draft-rejection"
needs = ["write"]

[steps.drafting]
kind = "loop"
body = ["write", "approve"]
until = { step = "approve", outcome = "approved" }

[steps.publish]
kind = "agent"
agent = "publisher"
needs = ["drafting"]
"#;

pub(super) const WRITER: &str = r#"
schema = 1
id = "writer"

[inputs.request]
format = "text"

[inputs.draft]
format = "markdown"
optional = true

[inputs.draft-rejection]
format = "text"
optional = true

[outputs.draft]
format = "markdown"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

pub(super) const PUBLISHER: &str = r#"
schema = 1
id = "publisher"

[inputs.draft]
format = "markdown"

[outputs.summary]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

pub(super) const ACTOR: &str = "dev@example.com";

pub(super) fn agent(toml: &'static str) -> AgentAssets {
    AgentAssets {
        toml,
        instructions: "Do the work.\n",
        schemas: &[],
    }
}

pub(super) fn review_catalog() -> Catalog {
    Catalog::new()
        .workflow("review", REVIEW_WORKFLOW)
        .agent("writer", agent(WRITER))
        .agent("publisher", agent(PUBLISHER))
}

pub(super) fn draft(text: &str) -> Behavior {
    Behavior::Deliver(Delivery::new("done").output("draft", "draft.md", text))
}

pub(super) fn summary(text: &str) -> Behavior {
    Behavior::Deliver(Delivery::new("done").output("summary", "summary.txt", text))
}

pub(super) fn label(key: &str, iteration: Option<u32>) -> StepLabel {
    StepLabel {
        key: key.to_owned(),
        item: None,
        iteration,
    }
}

pub(super) struct Driven {
    pub(super) outcome: Result<DriveOutcome, Error>,
    pub(super) progress: Vec<Progress>,
}

pub(super) struct Feature {
    pub(super) dir: ProjectDir,
    pub(super) project: Project,
    pub(super) run: RunId,
}

impl Feature {
    pub(super) async fn new(catalog: Catalog, workflow: &str) -> Feature {
        Feature::with_config(catalog, workflow, default_config()).await
    }

    pub(super) async fn with_config(catalog: Catalog, workflow: &str, config: Config) -> Feature {
        let dir = ProjectDir::new();
        let project = dir.project(catalog, config).await;
        let run = project
            .create_run(new_run(workflow))
            .await
            .expect("the run is created");
        Feature { dir, project, run }
    }

    pub(super) async fn review() -> Feature {
        Feature::new(review_catalog(), "review").await
    }

    pub(super) async fn drive(&self, script: Script) -> Driven {
        self.drive_with(&FakeRuntime::new(script)).await
    }

    pub(super) async fn drive_with(&self, runtime: &impl AgentRuntime) -> Driven {
        let (sink, mut events) = mpsc::unbounded_channel();
        let outcome = self
            .project
            .drive(&self.run, runtime, sink, StopSignal::new())
            .await;
        let mut progress = Vec::new();
        while let Ok(event) = events.try_recv() {
            if !matches!(event, Progress::Activity(_)) {
                progress.push(event);
            }
        }
        Driven { outcome, progress }
    }
}

pub(super) struct Recording {
    fake: FakeRuntime,
    requests: Mutex<Vec<RuntimeRequest>>,
}

impl Recording {
    pub(super) fn new(script: Script) -> Recording {
        Recording {
            fake: FakeRuntime::new(script),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn requests(&self) -> Vec<RuntimeRequest> {
        self.requests.lock().expect("the requests").clone()
    }
}

impl AgentRuntime for Recording {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        self.requests
            .lock()
            .expect("the requests")
            .push(preserved(&request));
        self.fake.execute(request, events, stop).await
    }
}

pub(super) fn preserved(request: &RuntimeRequest) -> RuntimeRequest {
    let mut kept = request.clone();
    let Some(execution) = request.delivery_dir.parent() else {
        return kept;
    };
    let (Some(id), Some(dot_surt)) = (execution.file_name(), execution.ancestors().nth(3)) else {
        return kept;
    };
    let copy = dot_surt.join("preserved").join(id);
    copy_tree(execution, &copy);
    if let Some(name) = request.delivery_dir.file_name() {
        kept.delivery_dir = copy.join(name);
    }
    kept.context = request.context.replace(
        execution.to_string_lossy().as_ref(),
        copy.to_string_lossy().as_ref(),
    );
    kept
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the copy directory");
    for entry in fs::read_dir(from).expect("the execution directory") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("the file is copied");
        }
    }
}
