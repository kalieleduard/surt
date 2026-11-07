import { column, row, step } from "./build";
import { TASKS, type TaskItem } from "./tasks";
import type { GraphNode } from "./types";

const MIDDLE_ROW = (row(0) + row(1)) / 2;

const ITEM_POSITIONS: Record<string, [number, number]> = {
  A: [column(0), MIDDLE_ROW],
  B: [column(1), row(0)],
  C: [column(1), row(1)],
  D: [column(2), MIDDLE_ROW],
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
    },
    "implement",
  );
}

export function itemNodes() {
  return TASKS.map(itemNode);
}
