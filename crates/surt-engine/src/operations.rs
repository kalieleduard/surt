pub(crate) mod checks;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    CreateRunBranch,
    CreateChildWorktree,
    RunChecks,
    Integrate,
    SnapshotRunBranch,
}

pub const BASE_SCHEMA: (&str, &str) = (
    "schemas/base.schema.json",
    include_str!("../assets/schemas/base.schema.json"),
);

pub const CONFLICT_SCHEMA: (&str, &str) = (
    "schemas/conflict.schema.json",
    include_str!("../assets/schemas/conflict.schema.json"),
);

pub const INTEGRATION_SCHEMA: (&str, &str) = (
    "schemas/integration.schema.json",
    include_str!("../assets/schemas/integration.schema.json"),
);

const REGISTRY: [Operation; 5] = [
    Operation::CreateRunBranch,
    Operation::CreateChildWorktree,
    Operation::RunChecks,
    Operation::Integrate,
    Operation::SnapshotRunBranch,
];

impl Operation {
    pub(crate) fn find(id: &str) -> Option<Operation> {
        REGISTRY.into_iter().find(|operation| operation.id() == id)
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Operation::CreateRunBranch => "git.create_run_branch",
            Operation::CreateChildWorktree => "git.create_child_worktree",
            Operation::RunChecks => "checks.run",
            Operation::Integrate => "git.integrate",
            Operation::SnapshotRunBranch => "git.snapshot_run_branch",
        }
    }

    pub(crate) fn inputs(self) -> &'static [&'static str] {
        match self {
            Operation::CreateRunBranch | Operation::SnapshotRunBranch => &["base"],
            Operation::CreateChildWorktree => &["item"],
            Operation::RunChecks => &["commit"],
            Operation::Integrate => &["commit", "feedback"],
        }
    }

    pub(crate) fn outcomes(self) -> &'static [&'static str] {
        match self {
            Operation::CreateRunBranch
            | Operation::CreateChildWorktree
            | Operation::SnapshotRunBranch => &["done"],
            Operation::RunChecks => &["passed", "failed"],
            Operation::Integrate => &["merged", "conflict"],
        }
    }

    pub(crate) fn produces(self) -> &'static [&'static str] {
        match self {
            Operation::CreateRunBranch | Operation::CreateChildWorktree => &[],
            Operation::RunChecks => &["check-report"],
            Operation::Integrate => &["conflict"],
            Operation::SnapshotRunBranch => &["integration"],
        }
    }
}
