import { describe, expect, it } from "vitest";
import type { FrameData, GraphNode, RunGraph, StepData } from "../data/types";
import { PROJECT, agent, approval, detail, event, gate, input, item, loop, operation, perItem, subject } from "./fixtures";
import { buildRunGraph, graphId } from "./runGraph";

const FEATURE = [
  agent("spec"),
  input("answer-spec", ["spec"], "spec-questions", { step: "spec", outcome: "questions" }),
  loop("spec-loop", [], ["spec", "answer-spec"], { step: "spec", outcome: "ready" }),
  approval("approve-spec", ["spec-loop"], "spec"),
  perItem("implement", ["approve-spec"], "tasks", "task-cycle"),
  operation("merge", ["implement"]),
];

const SPEC_TWICE = [
  event("step.running", "spec-loop"),
  event("iteration.started", "spec-loop", { data: { iteration: 1 } }),
  event("step.done", "spec", { iteration: 1, data: { outcome: "questions" } }),
  event("step.done", "answer-spec", { iteration: 1 }),
  event("iteration.started", "spec-loop", { data: { iteration: 2 } }),
  event("step.done", "spec", { iteration: 2, data: { outcome: "ready" } }),
  event("step.skipped", "answer-spec", { iteration: 2 }),
  event("step.done", "spec-loop"),
];

function node(graph: RunGraph, id: string): GraphNode {
  const found = graph.nodes.find((candidate) => candidate.id === id);
  if (!found) throw new Error(`no node ${id}`);
  return found;
}

const step = (graph: RunGraph, id: string) => node(graph, id).data as StepData;
const frame = (graph: RunGraph, id: string) => node(graph, id).data as FrameData;
const edge = (graph: RunGraph, id: string) => graph.edges.find((candidate) => candidate.id === id);

describe("buildRunGraph", () => {
  it("frames a loop around its body and a per item step around one node per Task", () => {
    const graph = buildRunGraph(
      PROJECT,
      subject({ itemStep: "implement", items: [item("A", "succeeded"), item("B", "pending")] }),
      null,
      detail(FEATURE, SPEC_TWICE),
    );

    expect(graph.nodes.map((entry) => [entry.id, entry.type, entry.parentId])).toEqual([
      ["spec-loop", "frame", undefined],
      ["spec", "step", "spec-loop"],
      ["answer-spec", "step", "spec-loop"],
      ["approve-spec", "step", undefined],
      ["implement", "frame", undefined],
      ["item:A", "step", "implement"],
      ["item:B", "step", "implement"],
      ["merge", "step", undefined],
    ]);
    expect(frame(graph, "spec-loop")).toMatchObject({ state: "succeeded", detail: "iter 2 · until spec ready" });
    expect(frame(graph, "implement").detail).toBe("task-cycle · 1/2");
    expect(step(graph, "answer-spec")).toMatchObject({ state: "skipped", condition: "spec = questions" });
  });

  it("enters a loop at its first body step, leaves it from its until step and loops back", () => {
    const graph = buildRunGraph(PROJECT, subject(), null, detail(FEATURE, SPEC_TWICE));

    expect(edge(graph, "spec->answer-spec")).toMatchObject({ state: "idle" });
    expect(edge(graph, "spec->approve-spec")).toMatchObject({ sourceHandle: "out:out", targetHandle: "in:in" });
    expect(edge(graph, "spec-loop:back")).toMatchObject({ source: "spec", target: "spec", state: "back", label: "iter 2" });
    expect(edge(graph, "implement->merge")).toMatchObject({ state: "idle" });
  });

  it("shows a reached gate as waiting with its summary and selects it", () => {
    const graph = buildRunGraph(
      PROJECT,
      subject({ states: ["awaiting_approval"], driven: false, gates: [gate("approve-spec", "approval", "spec", { revision: 2 })] }),
      null,
      detail(FEATURE, [...SPEC_TWICE, event("step.running", "approve-spec")]),
    );

    expect(step(graph, "approve-spec")).toMatchObject({ state: "waiting", artifact: "spec", gate: { revision: 2 } });
    expect(edge(graph, "spec->approve-spec")?.state).toBe("active");
    expect(graph.defaultSelection).toBe("approve-spec");
  });

  it("marks a running step of a run without a Driver as interrupted", () => {
    const graph = buildRunGraph(
      PROJECT,
      subject({ states: ["interrupted"], driven: false }),
      null,
      detail(FEATURE, [event("step.running", "spec-loop"), event("step.running", "spec", { iteration: 1 })]),
    );

    expect(step(graph, "spec").state).toBe("interrupted");
    expect(frame(graph, "spec-loop").state).toBe("interrupted");
  });

  it("draws Task edges from declared needs and names each Task by where it stands", () => {
    const tasks = [
      item("A", "succeeded", { mergedAs: "abc1234def" }),
      item("B", "running", { step: "build", iteration: 1, attempt: 2, needs: ["A"] }),
      item("C", "pending", { needs: ["B"], waitsFor: ["B"] }),
    ];
    const graph = buildRunGraph(PROJECT, subject({ itemStep: "implement", items: tasks }), null, detail(FEATURE, [event("step.running", "implement")]));

    expect(step(graph, "item:A")).toMatchObject({ name: "succeeded", summary: "merged abc1234" });
    expect(step(graph, "item:B")).toMatchObject({ name: "build", state: "running", summary: "iter 1 · attempt 2", workflow: "task-cycle" });
    expect(step(graph, "item:C")).toMatchObject({ name: "pending", summary: "waits for B" });
    expect(edge(graph, "item:A->item:B")?.state).toBe("active");
    expect(edge(graph, "item:B->item:C")?.state).toBe("idle");
    expect(frame(graph, "implement").state).toBe("running");
  });

  it("draws a Task's Child Workflow Run without the root run's Tasks and gates", () => {
    const cycle = [agent("setup"), agent("build", ["setup"])];
    const root = subject({ itemStep: "implement", items: [item("B", "running")], gates: [gate("approve-spec", "approval", "spec")] });

    const graph = buildRunGraph(PROJECT, root, "B", detail(cycle, [event("step.done", "setup", { data: { outcome: "done" } })]));

    expect(graph.id).toBe(graphId(root.name, "B"));
    expect(graph.path).toEqual(["shop", "20261107-login", "B"]);
    expect(graph.nodes.map((entry) => entry.id)).toEqual(["setup", "build"]);
    expect(graph.defaultSelection).toBe("setup");
  });
});
