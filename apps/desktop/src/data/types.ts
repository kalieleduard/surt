export type StepKind = "agent" | "approval" | "input" | "operation" | "loop" | "per_item" | "item";
export type StepState = "pending" | "running" | "waiting" | "succeeded" | "blocked" | "cancelled" | "skipped";
export type AgentId = "spec" | "tasks" | "builder" | "reviewer" | "shipper";

export type Port = { id: string; label: string; revision?: string; edge?: "top" | "bottom" };

export type Execution = {
  id: string;
  label: string;
  state: StepState;
  duration: string;
  note?: string;
};

export type StepData = {
  name: string;
  kind: StepKind;
  state: StepState;
  agent?: AgentId;
  operation?: string;
  outcome?: string;
  outcomes?: string[];
  attempt?: [number, number];
  iteration?: [number, number];
  duration?: string;
  inputs: Port[];
  outputs: Port[];
  executions?: Execution[];
  presence?: AgentId;
  itemKey?: string;
  title?: string;
  currentStep?: string;
  dependsOn?: string[];
  branch?: string;
  summary?: string;
};

export type FrameData = {
  name: string;
  kind: "loop" | "per_item";
  state: StepState;
  caption: string;
  detail: string;
  inputs?: Port[];
  outputs?: Port[];
};

export type GraphNode = {
  id: string;
  type: "step" | "frame";
  position: { x: number; y: number };
  parentId?: string;
  width?: number;
  height?: number;
  data: StepData | FrameData;
};

export type GraphEdge = {
  id: string;
  source: string;
  target: string;
  sourceHandle: string;
  targetHandle: string;
  label?: string;
  state: "done" | "active" | "idle" | "back";
};

export type RunGraph = {
  id: string;
  workflow: string;
  path: string[];
  nodes: GraphNode[];
  edges: GraphEdge[];
  defaultSelection: string;
};

export type Subject = {
  id: string;
  slug: string;
  workflow: string;
  state: StepState;
  status: string;
  children?: { key: string; title: string; state: StepState; status: string }[];
};

export const AGENT_LABEL: Record<AgentId, string> = {
  spec: "Spec Agent",
  tasks: "Task Agent",
  builder: "Builder",
  reviewer: "Reviewer",
  shipper: "Shipper",
};

export const AGENT_PROFILE: Record<AgentId, { runtime: string; model: string; effort: string; conversation: string }> = {
  spec: { runtime: "claude-code", model: "opus", effort: "high", conversation: "loop" },
  tasks: { runtime: "claude-code", model: "opus", effort: "high", conversation: "none" },
  builder: { runtime: "claude-code", model: "sonnet", effort: "medium", conversation: "loop" },
  reviewer: { runtime: "claude-code", model: "opus", effort: "high", conversation: "none" },
  shipper: { runtime: "claude-code", model: "sonnet", effort: "low", conversation: "none" },
};
