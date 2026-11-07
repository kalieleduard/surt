import { column, frame, frameSize, link, row, step } from "./build";
import { itemNodes } from "./items";
import type { RunGraph } from "./types";

const ROW_ONE = 0;
const ROW_TWO = 180;
const ROW_THREE = 476;

const specLoop = frame("spec-loop", 0, ROW_ONE, frameSize(3, 1), {
  name: "spec-loop",
  kind: "loop",
  state: "succeeded",
  caption: "loop · iteration 2/5",
  detail: "until approve-spec = approved",
});

const tasksLoop = frame("tasks-loop", 704, ROW_ONE, frameSize(2, 1), {
  name: "tasks-loop",
  kind: "loop",
  state: "succeeded",
  caption: "loop · iteration 1/5",
  detail: "until approved",
});

const implement = frame("implement", 232, ROW_TWO, frameSize(3, 2), {
  name: "implement",
  kind: "per_item",
  state: "running",
  caption: "per item · task-cycle",
  detail: "1/4 merged",
  inputs: [{ id: "manifest", label: "manifest r1", edge: "top" }, { id: "run-branch", label: "run branch" }],
  outputs: [{ id: "integration", label: "integration", edge: "bottom" }],
});

const steps = [
  step("spec", column(0), row(0), {
    name: "spec", kind: "agent", agent: "spec", state: "succeeded", outcome: "done", outcomes: ["questions", "done"],
    attempt: [1, 3], iteration: [2, 5], duration: "5m 02s",
    inputs: [{ id: "request", label: "request", revision: "r1" }, { id: "spec-answers", label: "spec-answers", revision: "r1" }],
    outputs: [{ id: "spec-questions", label: "spec-questions", revision: "r1" }, { id: "spec", label: "spec", revision: "r2" }],
    executions: [
      { id: "spec#1", label: "iteration 1 · attempt 1", state: "succeeded", duration: "4m 29s", note: "outcome questions" },
      { id: "spec#2", label: "iteration 2 · attempt 1", state: "succeeded", duration: "5m 02s", note: "continued conversation" },
    ],
  }, "spec-loop"),
  step("answer-spec", column(1), row(0), {
    name: "answer-spec", kind: "input", state: "succeeded", outcome: "answered", outcomes: ["answered"],
    duration: "6m 41s",
    inputs: [{ id: "spec-questions", label: "spec-questions", revision: "r1" }],
    outputs: [{ id: "spec-answers", label: "spec-answers", revision: "r1" }],
  }, "spec-loop"),
  step("approve-spec", column(2), row(0), {
    name: "approve-spec", kind: "approval", state: "succeeded", outcome: "approved", outcomes: ["approved", "rejected"],
    duration: "11m 10s",
    inputs: [{ id: "spec", label: "spec", revision: "r2" }],
    outputs: [{ id: "spec", label: "spec", revision: "r2 ✔" }],
  }, "spec-loop"),
  step("plan-tasks", column(0), row(0), {
    name: "plan-tasks", kind: "agent", agent: "tasks", state: "succeeded", outcome: "done", outcomes: ["done"],
    attempt: [1, 3], iteration: [1, 5], duration: "4m 48s",
    inputs: [{ id: "spec", label: "spec", revision: "r2" }],
    outputs: [{ id: "manifest", label: "manifest", revision: "r1" }],
    executions: [{ id: "plan#1", label: "iteration 1 · attempt 1", state: "succeeded", duration: "4m 48s" }],
  }, "tasks-loop"),
  step("approve-tasks", column(1), row(0), {
    name: "approve-tasks", kind: "approval", state: "succeeded", outcome: "approved", outcomes: ["approved", "rejected"],
    duration: "3m 05s",
    inputs: [{ id: "manifest", label: "manifest", revision: "r1" }],
    outputs: [{ id: "manifest", label: "manifest", revision: "r1 ✔" }],
  }, "tasks-loop"),
  step("prepare", 0, ROW_TWO + row(0) + 60, {
    name: "prepare", kind: "operation", operation: "git.create_run_branch", state: "succeeded", outcome: "created",
    duration: "0.4s",
    inputs: [{ id: "base", label: "base", revision: "b72ec4f" }],
    outputs: [{ id: "run-branch", label: "run branch", revision: "surt/20261107-3" }],
  }),
  step("snapshot", 560, ROW_THREE, {
    name: "snapshot", kind: "operation", operation: "git.snapshot_run_branch", state: "pending",
    inputs: [{ id: "integration", label: "integration" }],
    outputs: [{ id: "snapshot", label: "snapshot" }],
  }),
  step("ship", 780, ROW_THREE, {
    name: "ship", kind: "agent", agent: "shipper", state: "pending", outcomes: ["done"], attempt: [0, 3],
    inputs: [{ id: "snapshot", label: "snapshot" }, { id: "spec", label: "spec", revision: "r2" }],
    outputs: [{ id: "delivery", label: "delivery" }],
  }),
  step("merge", 1000, ROW_THREE, {
    name: "merge", kind: "operation", operation: "git.merge_into_base", state: "pending", outcomes: ["merged", "skipped"],
    inputs: [{ id: "delivery", label: "delivery" }],
    outputs: [{ id: "base", label: "master" }],
  }),
];

const edges = [
  link("spec", "spec-questions", "answer-spec", "spec-questions", "done"),
  link("answer-spec", "spec-answers", "spec", "spec-answers", "back"),
  link("spec", "spec", "approve-spec", "spec", "done"),
  link("approve-spec", "spec", "plan-tasks", "spec", "done", "spec r2"),
  link("plan-tasks", "manifest", "approve-tasks", "manifest", "done"),
  link("approve-tasks", "manifest", "implement", "manifest", "active", "items"),
  link("prepare", "run-branch", "implement", "run-branch", "active"),
  link("A", "merged", "B", "needs", "active"),
  link("A", "merged", "C", "needs", "active"),
  link("B", "merged", "D", "needs", "idle"),
  link("C", "merged", "D", "needs", "idle"),
  link("implement", "integration", "snapshot", "integration", "idle"),
  link("snapshot", "snapshot", "ship", "snapshot", "idle"),
  link("ship", "delivery", "merge", "delivery", "idle"),
];

export const FEATURE_GRAPH: RunGraph = {
  id: "feature",
  workflow: "feature",
  path: ["surt", "20261107-3", "feature"],
  nodes: [specLoop, tasksLoop, implement, ...steps, ...itemNodes()],
  edges,
  defaultSelection: "B",
};
