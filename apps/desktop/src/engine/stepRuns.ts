import type { Execution, Publication, StepState } from "../data/types";
import type { TimelineEvent } from "./types";

export type StepRun = {
  state: StepState;
  iteration: number | null;
  outcome: string | null;
  iterations: number;
  attempts: number;
  executions: Execution[];
  published: Publication[];
};

const STEP_STATE: Record<string, StepState> = {
  "step.running": "running",
  "step.done": "succeeded",
  "step.skipped": "skipped",
  "step.blocked": "blocked",
  "step.unblocked": "pending",
  "step.cancelled": "cancelled",
};

const EXECUTION_STATE: Record<string, StepState> = {
  "execution.succeeded": "succeeded",
  "execution.failed": "failed",
  "execution.interrupted": "interrupted",
  "execution.cancelled": "cancelled",
  "execution.orphan_stopped": "cancelled",
};

export function emptyStepRun(): StepRun {
  return { state: "pending", iteration: null, outcome: null, iterations: 0, attempts: 0, executions: [], published: [] };
}

export function foldStepRuns(events: TimelineEvent[]): Map<string, StepRun> {
  const runs = new Map<string, StepRun>();
  for (const event of events) {
    if (!event.step) continue;
    const run = runs.get(event.step) ?? emptyStepRun();
    runs.set(event.step, run);
    apply(run, event);
  }
  return runs;
}

function apply(run: StepRun, event: TimelineEvent) {
  const stepState = STEP_STATE[event.kind];
  if (stepState) return moveStep(run, event, stepState);
  if (event.kind === "iteration.started") run.iterations = Number(event.data.iteration) || run.iterations;
  if (event.kind === "execution.started") return startExecution(run, event);
  const executionState = EXECUTION_STATE[event.kind];
  if (executionState) return endExecution(run, event, executionState);
  if (event.kind === "revision.published") publish(run, event);
}

function moveStep(run: StepRun, event: TimelineEvent, state: StepState) {
  if (event.kind === "step.running" && event.iteration !== run.iteration) run.attempts = 0;
  run.state = state;
  run.iteration = event.iteration ?? run.iteration;
  run.outcome = event.kind === "step.done" ? text(event.data.outcome) : null;
}

function startExecution(run: StepRun, event: TimelineEvent) {
  if (!event.execution) return;
  run.attempts = event.attempt ?? run.attempts + 1;
  run.executions.push({
    id: event.execution,
    label: executionLabel(event),
    state: "running",
    startedAt: event.at,
    endedAt: null,
    note: null,
    runtime: text(event.data.runtime) ?? text(event.data.operation),
  });
}

function endExecution(run: StepRun, event: TimelineEvent, state: StepState) {
  const execution = run.executions.find((candidate) => candidate.id === event.execution);
  if (!execution) return;
  execution.state = state;
  execution.endedAt = event.at;
  execution.note = text(event.data.outcome) ?? text(event.data.message) ?? text(event.data.code);
}

function publish(run: StepRun, event: TimelineEvent) {
  const artifact = text(event.data.artifact);
  const revision = Number(event.data.revision);
  if (!artifact || !revision) return;
  run.published = [...run.published.filter((entry) => entry.artifact !== artifact), { artifact, revision }];
}

function executionLabel(event: TimelineEvent) {
  const attempt = `attempt ${event.attempt ?? 1}`;
  return event.iteration ? `iter ${event.iteration} · ${attempt}` : attempt;
}

function text(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}
