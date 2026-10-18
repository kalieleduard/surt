pub(crate) mod checks;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    CreateRunBranch,
    CreateChildWorktree,
    RunChecks,
    Integrate,
    SnapshotRunBranch,
    MergeIntoBase,
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

const REGISTRY: [Operation; 6] = [
    Operation::CreateRunBranch,
    Operation::CreateChildWorktree,
    Operation::RunChecks,
    Operation::Integrate,
    Operation::SnapshotRunBranch,
    Operation::MergeIntoBase,
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
            Operation::MergeIntoBase => "git.merge_into_base",
        }
    }

    pub(crate) fn inputs(self) -> &'static [&'static str] {
        match self {
            Operation::CreateRunBranch | Operation::SnapshotRunBranch => &["base"],
            Operation::CreateChildWorktree => &["item"],
            Operation::RunChecks => &["commit"],
            Operation::Integrate => &["commit", "feedback"],
            Operation::MergeIntoBase => &["base", "integration"],
        }
    }

    pub(crate) fn outcomes(self) -> &'static [&'static str] {
        match self {
            Operation::CreateRunBranch
            | Operation::CreateChildWorktree
            | Operation::SnapshotRunBranch => &["done"],
            Operation::RunChecks => &["passed", "failed"],
            Operation::Integrate => &["merged", "conflict"],
            Operation::MergeIntoBase => &["merged", "skipped"],
        }
    }

    pub(crate) fn produces(self) -> &'static [&'static str] {
        match self {
            Operation::CreateRunBranch
            | Operation::CreateChildWorktree
            | Operation::MergeIntoBase => &[],
            Operation::RunChecks => &["check-report"],
            Operation::Integrate => &["conflict"],
            Operation::SnapshotRunBranch => &["integration"],
        }
    }
}
