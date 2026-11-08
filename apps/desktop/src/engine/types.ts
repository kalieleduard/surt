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

export type RunsView = { slots: { inUse: number; total: number }; subjects: SubjectView[] };

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
