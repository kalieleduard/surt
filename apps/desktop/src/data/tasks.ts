import type { AgentId, StepState } from "./types";

export type TaskItem = {
  key: string;
  title: string;
  state: StepState;
  currentStep: string;
  status: string;
  agent?: AgentId;
  attempt?: [number, number];
  iteration: [number, number];
  dependsOn: string[];
  diff?: string;
};

export const TASKS: TaskItem[] = [
  {
    key: "A",
    title: "OAuth provider config",
    state: "succeeded",
    currentStep: "integrate",
    status: "merged",
    iteration: [1, 5],
    dependsOn: [],
    diff: "+184 −12",
  },
  {
    key: "B",
    title: "Callback handler",
    state: "running",
    currentStep: "build",
    status: "build · attempt 2/3",
    agent: "builder",
    attempt: [2, 3],
    iteration: [2, 5],
    dependsOn: ["A"],
    diff: "+96 −8",
  },
  {
    key: "C",
    title: "Session cookies",
    state: "running",
    currentStep: "review",
    status: "review · iteration 1",
    agent: "reviewer",
    attempt: [1, 3],
    iteration: [1, 5],
    dependsOn: ["A"],
    diff: "+142 −37",
  },
  {
    key: "D",
    title: "Login screen",
    state: "pending",
    currentStep: "setup",
    status: "waits for B",
    iteration: [0, 5],
    dependsOn: ["B", "C"],
  },
];

export function taskByKey(key: string) {
  return TASKS.find((task) => task.key === key) ?? TASKS[0];
}
