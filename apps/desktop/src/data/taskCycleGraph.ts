import { frame, link, step } from "./build";
import { taskByKey, type TaskItem } from "./tasks";
import type { GraphEdge, RunGraph, StepData, StepState } from "./types";

const CYCLE_ORDER = ["setup", "build", "check", "review", "integrate"];

function stateOf(task: TaskItem, stepName: string): StepState {
  if (task.state === "succeeded") return "succeeded";
  const current = CYCLE_ORDER.indexOf(task.currentStep);
  const index = CYCLE_ORDER.indexOf(stepName);
  if (index < current) return "succeeded";
  if (index === current) return task.state;
  return "pending";
}

function edgeState(task: TaskItem, target: string): GraphEdge["state"] {
  const state = stateOf(task, target);
  if (state === "running") return "active";
  return state === "succeeded" ? "done" : "idle";
}

function cycleSteps(task: TaskItem): Record<string, StepData> {
  const iteration = task.iteration;
  const builderTail = task.key === "B"
    ? ["↻ attempt 2 after invalid output", "$ cargo test -p auth callback", "running 18 tests", "test callback::state_mismatch ... ok"]
    : ["✔ commit 3f9a1c2 delivered", "outcome done"];
  return {
    setup: {
      name: "setup", kind: "operation", operation: "git.create_child_worktree", state: stateOf(task, "setup"),
      inputs: [{ id: "run-branch", label: "run branch" }],
      outputs: [{ id: "worktree", label: "worktree", revision: task.key }],
      tail: ["$ git worktree add .surt/worktrees/" + task.key, "✔ worktree ready"],
    },
    build: {
      name: "build", kind: "agent", agent: "builder", state: stateOf(task, "build"), outcomes: ["done"],
      attempt: task.key === "B" ? [2, 3] : [1, 3], iteration, presence: task.currentStep === "build" ? "builder" : undefined,
      inputs: [
        { id: "worktree", label: "worktree" },
        { id: "task", label: `task ${task.key}.md`, revision: "r1" },
        { id: "feedback", label: "feedback", revision: task.key === "B" ? "r1" : undefined },
      ],
      outputs: [{ id: "commit", label: "commit", revision: task.key === "B" ? "pending" : "3f9a1c2" }],
      tail: builderTail,
      executions: task.key === "B"
        ? [
            { id: "build#1", label: "iteration 1 · attempt 1", state: "succeeded", duration: "9m 12s", note: "commit 81c0d4e" },
            { id: "build#2", label: "iteration 2 · attempt 1", state: "blocked", duration: "4m 12s", note: "invalid output: missing commit" },
            { id: "build#3", label: "iteration 2 · attempt 2", state: "running", duration: "3m 40s", note: "continues conversation" },
          ]
        : [{ id: "build#1", label: "iteration 1 · attempt 1", state: "succeeded", duration: "11m 03s" }],
    },
    check: {
      name: "check", kind: "operation", operation: "checks.run", state: stateOf(task, "check"), outcomes: ["passed", "failed"],
      outcome: stateOf(task, "check") === "succeeded" ? "passed" : undefined,
      inputs: [{ id: "commit", label: "commit" }],
      outputs: [{ id: "check-report", label: "check-report" }],
      tail: ["$ cargo test --workspace", "test result: ok. 212 passed"],
    },
    review: {
      name: "review", kind: "agent", agent: "reviewer", state: stateOf(task, "review"), outcomes: ["approved", "changes_requested"],
      attempt: [1, 3], iteration, presence: task.currentStep === "review" ? "reviewer" : undefined,
      inputs: [{ id: "commit", label: "commit" }, { id: "check-report", label: "check-report" }],
      outputs: [{ id: "feedback", label: "feedback" }, { id: "approved", label: "approved commit" }],
      tail: ["● reading commit 3f9a1c2", "findings so far: 1 minor", "src/session/cookie.rs:42"],
    },
    integrate: {
      name: "integrate", kind: "operation", operation: "git.integrate", state: stateOf(task, "integrate"), outcomes: ["merged", "conflict"],
      outcome: task.state === "succeeded" ? "merged" : undefined,
      inputs: [{ id: "approved", label: "approved commit" }],
      outputs: [{ id: "conflict", label: "conflict" }, { id: "merged", label: "integration" }],
      tail: ["⟳ merge --no-ff into surt/20261107-3", "run checks on the merge"],
    },
  };
}

export function taskCycleGraph(key: string): RunGraph {
  const task = taskByKey(key);
  const data = cycleSteps(task);
  const cycle = frame("cycle", 260, 0, [770, 420], {
    name: "cycle",
    kind: "loop",
    state: task.state,
    caption: `loop · iteration ${Math.max(task.iteration[0], 1)}/${task.iteration[1]}`,
    detail: "until integrate = merged",
  });
  return {
    id: `task-cycle:${task.key}`,
    workflow: "task-cycle",
    path: ["surt", "20261107-3", "implement", task.key],
    nodes: [
      cycle,
      step("setup", 0, 56, data.setup),
      step("build", 20, 56, data.build, "cycle"),
      step("check", 270, 56, data.check, "cycle"),
      step("review", 520, 56, data.review, "cycle"),
      step("integrate", 520, 240, data.integrate, "cycle"),
    ],
    edges: [
      link("setup", "worktree", "build", "worktree", edgeState(task, "build")),
      link("build", "commit", "check", "commit", edgeState(task, "check")),
      link("check", "check-report", "review", "check-report", edgeState(task, "review")),
      link("review", "feedback", "build", "feedback", task.key === "B" ? "back" : "idle", "changes_requested"),
      link("review", "approved", "integrate", "approved", edgeState(task, "integrate")),
      link("integrate", "conflict", "build", "feedback", "idle", "conflict"),
    ],
    defaultSelection: task.currentStep === "setup" ? "setup" : task.currentStep,
  };
}
