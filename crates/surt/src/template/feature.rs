use surt_engine::project::{BASE_SCHEMA, CHECK_REPORT_SCHEMA, CONFLICT_SCHEMA, INTEGRATION_SCHEMA};

use super::{AgentFiles, Template};

const MANIFEST_SCHEMA: (&str, &str) = (
    "schemas/manifest.schema.json",
    include_str!("../../assets/templates/feature/agents/tasks/schemas/manifest.schema.json"),
);
const COMMIT_SCHEMA: (&str, &str) = (
    "schemas/commit.schema.json",
    include_str!("../../assets/templates/feature/agents/builder/schemas/commit.schema.json"),
);
const FEEDBACK_SCHEMA: (&str, &str) = (
    "schemas/feedback.schema.json",
    include_str!("../../assets/templates/feature/agents/reviewer/schemas/feedback.schema.json"),
);
const DELIVERY_SCHEMA: (&str, &str) = (
    "schemas/delivery.schema.json",
    include_str!("../../assets/templates/feature/agents/shipper/schemas/delivery.schema.json"),
);

const SPEC: AgentFiles = &[
    (
        "agent.toml",
        include_str!("../../assets/templates/feature/agents/spec/agent.toml"),
    ),
    (
        "instructions.md",
        include_str!("../../assets/templates/feature/agents/spec/instructions.md"),
    ),
];

const TASKS: AgentFiles = &[
    (
        "agent.toml",
        include_str!("../../assets/templates/feature/agents/tasks/agent.toml"),
    ),
    (
        "instructions.md",
        include_str!("../../assets/templates/feature/agents/tasks/instructions.md"),
    ),
    MANIFEST_SCHEMA,
];

const BUILDER: AgentFiles = &[
    (
        "agent.toml",
        include_str!("../../assets/templates/feature/agents/builder/agent.toml"),
    ),
    (
        "instructions.md",
        include_str!("../../assets/templates/feature/agents/builder/instructions.md"),
    ),
    COMMIT_SCHEMA,
    FEEDBACK_SCHEMA,
    CHECK_REPORT_SCHEMA,
    CONFLICT_SCHEMA,
];

const REVIEWER: AgentFiles = &[
    (
        "agent.toml",
        include_str!("../../assets/templates/feature/agents/reviewer/agent.toml"),
    ),
    (
        "instructions.md",
        include_str!("../../assets/templates/feature/agents/reviewer/instructions.md"),
    ),
    FEEDBACK_SCHEMA,
    COMMIT_SCHEMA,
    CHECK_REPORT_SCHEMA,
    CONFLICT_SCHEMA,
];

const SHIPPER: AgentFiles = &[
    (
        "agent.toml",
        include_str!("../../assets/templates/feature/agents/shipper/agent.toml"),
    ),
    (
        "instructions.md",
        include_str!("../../assets/templates/feature/agents/shipper/instructions.md"),
    ),
    DELIVERY_SCHEMA,
    MANIFEST_SCHEMA,
    INTEGRATION_SCHEMA,
    BASE_SCHEMA,
];

pub(super) const TEMPLATE: Template = Template {
    workflows: &[
        (
            "feature.toml",
            include_str!("../../assets/templates/feature/workflows/feature.toml"),
        ),
        (
            "task-cycle.toml",
            include_str!("../../assets/templates/feature/workflows/task-cycle.toml"),
        ),
    ],
    agents: &[
        ("spec", SPEC),
        ("tasks", TASKS),
        ("builder", BUILDER),
        ("reviewer", REVIEWER),
        ("shipper", SHIPPER),
    ],
    next: "surt run feature --input request=\"<what you want to build>\"",
};
