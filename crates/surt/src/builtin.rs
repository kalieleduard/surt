use surt_engine::project::{
    AgentAssets, BASE_SCHEMA, CHECK_REPORT_SCHEMA, CONFLICT_SCHEMA, Catalog, INTEGRATION_SCHEMA,
};

pub(crate) const FEATURE_WORKFLOW: &str = "feature";
pub(crate) const REQUEST_ARTIFACT: &str = "request";
pub(crate) const BASE_ARTIFACT: &str = "base";

const TASK_CYCLE_WORKFLOW: &str = "task-cycle";
const BUILDER: &str = "builder";
const REVIEWER: &str = "reviewer";

const MANIFEST_SCHEMA: (&str, &str) = (
    "schemas/manifest.schema.json",
    include_str!("../assets/agents/tasks/schemas/manifest.schema.json"),
);
const COMMIT_SCHEMA: (&str, &str) = (
    "schemas/commit.schema.json",
    include_str!("../assets/agents/builder/schemas/commit.schema.json"),
);
const FEEDBACK_SCHEMA: (&str, &str) = (
    "schemas/feedback.schema.json",
    include_str!("../assets/agents/reviewer/schemas/feedback.schema.json"),
);
const DELIVERY_SCHEMA: (&str, &str) = (
    "schemas/delivery.schema.json",
    include_str!("../assets/agents/shipper/schemas/delivery.schema.json"),
);

const AGENTS: [(&str, AgentAssets); 5] = [
    (
        "spec",
        AgentAssets {
            toml: include_str!("../assets/agents/spec/agent.toml"),
            instructions: include_str!("../assets/agents/spec/instructions.md"),
            schemas: &[],
        },
    ),
    (
        "tasks",
        AgentAssets {
            toml: include_str!("../assets/agents/tasks/agent.toml"),
            instructions: include_str!("../assets/agents/tasks/instructions.md"),
            schemas: &[MANIFEST_SCHEMA],
        },
    ),
    (
        BUILDER,
        AgentAssets {
            toml: include_str!("../assets/agents/builder/agent.toml"),
            instructions: include_str!("../assets/agents/builder/instructions.md"),
            schemas: &[
                COMMIT_SCHEMA,
                FEEDBACK_SCHEMA,
                CHECK_REPORT_SCHEMA,
                CONFLICT_SCHEMA,
            ],
        },
    ),
    (
        REVIEWER,
        AgentAssets {
            toml: include_str!("../assets/agents/reviewer/agent.toml"),
            instructions: include_str!("../assets/agents/reviewer/instructions.md"),
            schemas: &[
                FEEDBACK_SCHEMA,
                COMMIT_SCHEMA,
                CHECK_REPORT_SCHEMA,
                CONFLICT_SCHEMA,
            ],
        },
    ),
    (
        "shipper",
        AgentAssets {
            toml: include_str!("../assets/agents/shipper/agent.toml"),
            instructions: include_str!("../assets/agents/shipper/instructions.md"),
            schemas: &[
                DELIVERY_SCHEMA,
                MANIFEST_SCHEMA,
                INTEGRATION_SCHEMA,
                BASE_SCHEMA,
            ],
        },
    ),
];

pub(crate) fn catalog() -> Catalog {
    let workflows = Catalog::new()
        .workflow(
            FEATURE_WORKFLOW,
            include_str!("../assets/workflows/feature.toml"),
        )
        .workflow(
            TASK_CYCLE_WORKFLOW,
            include_str!("../assets/workflows/task-cycle.toml"),
        );
    AGENTS
        .into_iter()
        .fold(workflows, |catalog, (id, assets)| catalog.agent(id, assets))
}

#[cfg(test)]
mod tests;
