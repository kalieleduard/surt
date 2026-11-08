import type { DefinedStep, GateSummary } from "../engine/types";

export type StepKind = "agent" | "approval" | "input" | "operation" | "loop" | "per_item" | "item";
export type StepState =
  | "pending"
  | "running"
  | "waiting"
  | "succeeded"
  | "blocked"
  | "cancelled"
  | "skipped"
  | "interrupted"
  | "failed";

export type Port = { id: string; label: string; y?: number };

export type Execution = {
  id: string;
  label: string;
  state: StepState;
  startedAt: number;
  endedAt: number | null;
  note: string | null;
  runtime: string | null;
};

export type Publication = { artifact: string; revision: number };

export type StepData = {
  key: string;
  name: string;
  kind: StepKind;
  state: StepState;
  agent?: string;
  operation?: string;
  artifact?: string;
  outcome?: string;
  condition?: string;
  iteration?: number;
  attempts: number;
  inputs: Port[];
  outputs: Port[];
  executions: Execution[];
  published: Publication[];
  gate?: GateSummary;
  itemKey?: string;
  workflow?: string;
  currentStep?: string;
  needs?: string[];
  waitsFor?: string[];
  mergedAs?: string;
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
  entry?: string;
  exit?: string;
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
  frame?: string;
  state: "done" | "active" | "idle" | "back";
};

export type RunGraph = {
  id: string;
  subject: string;
  item: string | null;
  run: number;
  workflow: string;
  source: string;
  path: string[];
  nodes: GraphNode[];
  edges: GraphEdge[];
  steps: DefinedStep[];
  defaultSelection: string;
  preview?: boolean;
};
