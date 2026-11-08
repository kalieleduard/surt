import { describe, expect, it } from "vitest";
import { event } from "./fixtures";
import { foldStepRuns } from "./stepRuns";

describe("foldStepRuns", () => {
  it("reports the latest iteration and outcome of a step that ran twice in a loop", () => {
    const runs = foldStepRuns([
      event("iteration.started", "spec-loop", { data: { iteration: 1 } }),
      event("step.running", "spec", { iteration: 1 }),
      event("step.done", "spec", { iteration: 1, data: { outcome: "questions" } }),
      event("iteration.started", "spec-loop", { data: { iteration: 2 } }),
      event("step.running", "spec", { iteration: 2 }),
      event("step.done", "spec", { iteration: 2, data: { outcome: "ready" } }),
    ]);

    expect(runs.get("spec")).toMatchObject({ state: "succeeded", iteration: 2, outcome: "ready" });
    expect(runs.get("spec-loop")?.iterations).toBe(2);
  });

  it("counts attempts within an iteration and starts again at the next iteration", () => {
    const runs = foldStepRuns([
      event("step.running", "build", { iteration: 1 }),
      event("execution.started", "build", { iteration: 1, attempt: 1, execution: "e1", at: 100, data: { runtime: "claude" } }),
      event("execution.failed", "build", { execution: "e1", at: 160, data: { code: "exit_1" } }),
      event("execution.started", "build", { iteration: 1, attempt: 2, execution: "e2", at: 170 }),
      event("execution.succeeded", "build", { execution: "e2", at: 200, data: { outcome: "done" } }),
      event("step.done", "build", { iteration: 1, data: { outcome: "done" } }),
      event("step.running", "build", { iteration: 2 }),
    ]);

    const build = runs.get("build");
    expect(build?.state).toBe("running");
    expect(build?.attempts).toBe(0);
    expect(build?.outcome).toBeNull();
    expect(build?.executions).toEqual([
      { id: "e1", label: "iter 1 · attempt 1", state: "failed", startedAt: 100, endedAt: 160, note: "exit_1", runtime: "claude" },
      { id: "e2", label: "iter 1 · attempt 2", state: "succeeded", startedAt: 170, endedAt: 200, note: "done", runtime: null },
    ]);
  });

  it("keeps only the latest published revision of each artifact", () => {
    const runs = foldStepRuns([
      event("revision.published", "spec", { data: { artifact: "spec", revision: 1 } }),
      event("revision.published", "spec", { data: { artifact: "spec-questions", revision: 1 } }),
      event("revision.published", "spec", { data: { artifact: "spec", revision: 2 } }),
      event("revision.published", "spec", { data: { artifact: "", revision: 3 } }),
    ]);

    expect(runs.get("spec")?.published).toEqual([
      { artifact: "spec-questions", revision: 1 },
      { artifact: "spec", revision: 2 },
    ]);
  });

  it("returns a blocked step to pending once it is unblocked and ignores run-wide events", () => {
    const runs = foldStepRuns([
      event("run.started", null),
      event("step.running", "check", { iteration: 1 }),
      event("step.blocked", "check", { iteration: 1 }),
      event("step.unblocked", "check", { iteration: 1 }),
      event("execution.succeeded", "check", { execution: "unknown" }),
    ]);

    expect([...runs.keys()]).toEqual(["check"]);
    expect(runs.get("check")).toMatchObject({ state: "pending", iteration: 1, executions: [] });
  });
});
