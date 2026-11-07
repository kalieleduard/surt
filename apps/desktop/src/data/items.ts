import { step } from "./build";
import { TASKS, type TaskItem } from "./tasks";
import type { GraphNode } from "./types";

const ITEM_POSITIONS: Record<string, [number, number]> = {
  A: [20, 136],
  B: [270, 56],
  C: [270, 224],
  D: [520, 136],
};

const ITEM_TAIL: Record<string, string[]> = {
  A: ["✔ review approved", "⟳ integrate · merge --no-ff", "✔ merged +184 −12"],
  B: ["↻ attempt 2 after invalid output", "$ cargo test -p auth callback", "running 18 tests", "test callback::state_mismatch ... ok"],
  C: ["● review started 4m ago", "reading commit 3f9a1c2", "check-report: 212 passed"],
  D: ["⏳ waits for B and C"],
};

function itemNode(task: TaskItem): GraphNode {
  const [x, y] = ITEM_POSITIONS[task.key];
  return step(
    task.key,
    x,
    y,
    {
      name: `${task.key} · ${task.title}`,
      kind: "item",
      state: task.state,
      agent: task.agent,
      itemKey: task.key,
      title: task.title,
      currentStep: task.currentStep,
      summary: task.status,
      attempt: task.attempt,
      iteration: task.iteration,
      dependsOn: task.dependsOn,
      branch: `surt/item/20261107-3/${task.key}`,
      presence: task.agent,
      inputs: [{ id: "needs", label: task.dependsOn.length ? `needs ${task.dependsOn.join(", ")}` : "base" }],
      outputs: [{ id: "merged", label: task.diff ?? "commit" }],
      tail: ITEM_TAIL[task.key],
    },
    "implement",
  );
}

export function itemNodes() {
  return TASKS.map(itemNode);
}
