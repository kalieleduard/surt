export type RunStateName =
  | "running"
  | "awaiting_approval"
  | "awaiting_answer"
  | "blocked"
  | "interrupted"
  | "succeeded"
  | "cancelled";

export type ItemStateName = "pending" | "queued" | "running" | "interrupted" | "blocked" | "succeeded" | "cancelled";

export type ProjectView = { root: string; name: string; maxAttempts: number; maxIterations: number };

export type ItemView = {
  key: string;
  state: ItemStateName;
  step: string | null;
  iteration: number | null;
  attempt: number | null;
  since: number | null;
  mergedAs: string | null;
  needs: string[];
  waitsFor: string[];
};

export type GateSummary = {
  step: string;
  kind: "approval" | "input";
  artifact: string;
  status: "not_reached" | "waiting" | "approved" | "rejected" | "answered";
  revision: number | null;
  decidedBy: string | null;
  decidedAt: number | null;
};

export type BlockedView = {
  step: string;
  item: string | null;
  iteration: number | null;
  reason: string;
  failure: string | null;
};

export type SubjectView = {
  name: string;
  run: number;
  runs: number;
  states: RunStateName[];
  endedAt: number | null;
  driven: boolean;
  branch: string | null;
  itemStep: string | null;
  items: ItemView[];
  gates: GateSummary[];
  blocked: BlockedView[];
};

export type DriverEnding = {
  outcome: "at_gate" | "succeeded" | "blocked" | "cancelled" | "interrupted" | "refused" | "failed";
  message: string | null;
};

export type DriverView = { subject: string; driving: boolean; ending: DriverEnding | null };

export type RunsView = { slots: { inUse: number; total: number }; subjects: SubjectView[]; drivers: DriverView[] };

export type Choice = { value: string; description: string | null };

export type QuestionView = { id: string; question: string; why: string; recommended: string; options: Choice[] | null };

export type WaitingInput = { step: string; artifact: string; revision: number; round: number; questions: QuestionView[] };

export type CancelImpact = { executions: number; items: number; merged: number; integrationBranch: string | null };

export type RunInput = {
  name: string;
  format: "text" | "markdown" | "json" | "git_ref";
  optional: boolean;
  default: string | null;
  description: string | null;
};

export type WorkflowView = { name: string; runInputs: RunInput[] | null; problem: string | null };

export type Condition = { step: string; outcome: string };

type StepShape =
  | { kind: "agent"; agent: string }
  | { kind: "approval"; artifact: string }
  | { kind: "input"; artifact: string }
  | { kind: "operation"; operation: string }
  | { kind: "loop"; body: string[]; until: Condition }
  | { kind: "per_item"; items: string; workflow: string };

export type DefinedStep = { key: string; needs: string[]; when: Condition | null } & StepShape;

export type TimelineEvent = {
  at: number;
  kind: string;
  item: string | null;
  step: string | null;
  iteration: number | null;
  attempt: number | null;
  execution: string | null;
  data: Record<string, unknown>;
};

export type RunDetail = {
  definition: { name: string; source: string; steps: DefinedStep[] };
  events: TimelineEvent[];
};

export type AgentEntry = { id: string; files: string[] };

export type WorkflowEntry = { name: string; file: string; problem: string | null };

export type Catalog = { agents: AgentEntry[]; workflows: WorkflowEntry[] };

export type WorktreeView = { path: string; name: string; branch: string | null; head: string; main: boolean };

export type BranchView = { name: string; worktree: string | null };

export type Checkouts = { worktrees: WorktreeView[]; branches: BranchView[] };

export type ScriptView = { name: string; description: string | null; command: string; running: boolean };

export type ScriptOutput = { script: string; stream: "stdout" | "stderr"; line: string };

export type ScriptEnded = { script: string; code: number | null; stopped: boolean; error: string | null };
