export type OperationSpec = { summary: string; inputs: string[]; produces: string[]; outcomes: string[] };

export const OPERATIONS: Record<string, OperationSpec> = {
  "git.create_run_branch": {
    summary: "Creates the run branch from the base ref.",
    inputs: ["base"],
    produces: [],
    outcomes: ["done"],
  },
  "git.create_child_worktree": {
    summary: "Creates this item's branch and worktree from the run branch tip.",
    inputs: ["item"],
    produces: [],
    outcomes: ["done"],
  },
  "checks.run": {
    summary: "Runs the project's checks against the delivered commit.",
    inputs: ["commit"],
    produces: ["check-report"],
    outcomes: ["passed", "failed"],
  },
  "git.integrate": {
    summary: "Integrates the reviewed commit into the run branch.",
    inputs: ["commit", "feedback"],
    produces: ["conflict"],
    outcomes: ["merged", "conflict"],
  },
  "git.snapshot_run_branch": {
    summary: "Records the run branch tip as the integration result.",
    inputs: ["base"],
    produces: ["integration"],
    outcomes: ["done"],
  },
  "git.merge_into_base": {
    summary: "Merges the run branch into the base branch, when the run allows it.",
    inputs: ["base", "integration"],
    produces: [],
    outcomes: ["merged", "skipped"],
  },
};
