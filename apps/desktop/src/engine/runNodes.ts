import type { FrameData, GraphNode, StepData, StepState } from "../data/types";
import { FRAME_HEADER, FRAME_INSET, LOOP_LANE, NODE_BOX, arrange, framed, layers, stacked, type Arrangement, type Box, type Point } from "./arrange";
import { emptyStepRun, type StepRun } from "./stepRuns";
import type { DefinedStep, GateSummary, ItemView } from "./types";

export const IN = "in";
export const OUT = "out";
export const ITEM_PREFIX = "item:";
const MAX_ROW_WIDTH = 1400;

export type RunSource = {
  steps: Map<string, DefinedStep>;
  runs: Map<string, StepRun>;
  driven: boolean;
  items: ItemView[];
  itemStep: string | null;
  gates: GateSummary[];
};

export function needsOf(source: RunSource) {
  return (key: string) => source.steps.get(key)?.needs ?? [];
}

export function runOf(source: RunSource, key: string): StepRun {
  return source.runs.get(key) ?? emptyStepRun();
}

export function topLevel(source: RunSource): string[] {
  const inside = new Set([...source.steps.values()].flatMap((step) => (step.kind === "loop" ? step.body : [])));
  return [...source.steps.keys()].filter((key) => !inside.has(key));
}

export function runNodes(source: RunSource): GraphNode[] {
  const layout = new Layout(source);
  const top = arrange(stacked(layers(topLevel(source), needsOf(source))), layout.size, MAX_ROW_WIDTH);
  const nodes: GraphNode[] = [];
  for (const [key, position] of top.positions) layout.emit(key, position, undefined, nodes);
  return nodes;
}

class Layout {
  private boxes = new Map<string, Box>();
  private inner = new Map<string, Arrangement>();
  private source: RunSource;

  constructor(source: RunSource) {
    this.source = source;
  }

  size = (key: string): Box => {
    const known = this.boxes.get(key);
    if (known) return known;
    const box = this.measure(key);
    this.boxes.set(key, box);
    return box;
  };

  emit(key: string, position: Point, parentId: string | undefined, nodes: GraphNode[]) {
    const step = this.source.steps.get(key);
    if (!step) return;
    if (step.kind === "loop" || step.kind === "per_item") return this.emitFrame(step, position, parentId, nodes);
    nodes.push({ id: key, type: "step", position, parentId, data: stepData(step, this.source) });
  }

  private measure(key: string): Box {
    const step = this.source.steps.get(key);
    if (step?.kind === "loop") return framed(this.arrangement(key, step.body, this.size).box, LOOP_LANE);
    if (step?.kind === "per_item") return framed(this.itemArrangement(step).box);
    return NODE_BOX;
  }

  private arrangement(key: string, members: string[], sizeOf: (member: string) => Box): Arrangement {
    const known = this.inner.get(key);
    if (known) return known;
    const needs = key === this.source.itemStep ? itemNeeds(this.source.items) : needsOf(this.source);
    const arranged = members.length ? arrange(stacked(layers(members, needs)), sizeOf) : { positions: new Map(), box: NODE_BOX };
    this.inner.set(key, arranged);
    return arranged;
  }

  private itemArrangement(step: DefinedStep): Arrangement {
    const items = step.key === this.source.itemStep ? this.source.items.map((item) => item.key) : [];
    return this.arrangement(step.key, items, () => NODE_BOX);
  }

  private emitFrame(step: DefinedStep, position: Point, parentId: string | undefined, nodes: GraphNode[]) {
    const box = this.size(step.key);
    const at = nodes.length;
    const frame = frameData(step, this.source);
    frame.inputs = frame.inputs?.map((port) => ({ ...port, y: this.portY(step, "in") }));
    frame.outputs = frame.outputs?.map((port) => ({ ...port, y: this.portY(step, "out") }));
    if (step.kind === "loop") {
      frame.entry = this.entryOf(step);
      frame.exit = step.until.step;
    }
    nodes.push({ id: step.key, type: "frame", position, parentId, width: box.width, height: box.height, data: frame });
    for (const [member, inner] of this.inner.get(step.key)?.positions ?? []) {
      const offset = { x: inner.x + FRAME_INSET, y: inner.y + FRAME_HEADER };
      if (step.kind === "loop") this.emit(member, offset, step.key, nodes);
      else nodes.push(itemNode(member, offset, step, this.source));
    }
    frame.state = frameState(runOf(this.source, step.key), nodes.slice(at + 1), this.source.driven);
  }

  private portY(step: DefinedStep, side: "in" | "out"): number {
    const positions = this.inner.get(step.key)?.positions;
    const member = side === "in" ? this.entryOf(step) : step.kind === "loop" ? step.until.step : undefined;
    const at = member ? positions?.get(member) : undefined;
    const tops = [...(positions?.values() ?? [])].map((point) => point.y);
    if (!member || !at) return FRAME_HEADER + (tops.length ? Math.min(...tops) : 0) + NODE_BOX.height / 2;
    const inner = this.source.steps.get(member);
    const offset = inner?.kind === "loop" || inner?.kind === "per_item" ? this.portY(inner, side) : NODE_BOX.height / 2;
    return FRAME_HEADER + at.y + offset;
  }

  private entryOf(step: DefinedStep): string | undefined {
    if (step.kind === "loop") return layers(step.body, needsOf(this.source))[0]?.[0];
    return this.inner.get(step.key)?.positions.keys().next().value;
  }
}

function itemNeeds(items: ItemView[]) {
  return (key: string) => items.find((item) => item.key === key)?.needs ?? [];
}

export function stepState(step: DefinedStep, run: StepRun, driven: boolean): StepState {
  if (run.state !== "running") return run.state;
  if (step.kind === "approval" || step.kind === "input") return "waiting";
  return driven ? "running" : "interrupted";
}

function frameState(run: StepRun, children: GraphNode[], driven: boolean): StepState {
  if (run.state !== "running") return run.state;
  const states = children.map((child) => (child.data as StepData).state);
  if (states.includes("waiting")) return "waiting";
  if (states.includes("blocked")) return "blocked";
  return driven ? "running" : "interrupted";
}

function ports() {
  return { inputs: [{ id: IN, label: "" }], outputs: [{ id: OUT, label: "" }] };
}

function stepData(step: DefinedStep, source: RunSource): StepData {
  const run = runOf(source, step.key);
  const base: StepData = {
    key: step.key,
    name: step.key,
    kind: step.kind,
    state: stepState(step, run, source.driven),
    outcome: run.outcome ?? undefined,
    condition: step.when ? `${step.when.step} = ${step.when.outcome}` : undefined,
    iteration: run.iteration ?? undefined,
    attempts: run.attempts,
    executions: run.executions,
    published: run.published,
    ...ports(),
  };
  if (step.kind === "agent") return { ...base, agent: step.agent };
  if (step.kind === "operation") return { ...base, operation: step.operation };
  if (step.kind === "approval" || step.kind === "input") {
    return { ...base, artifact: step.artifact, gate: source.gates.find((gate) => gate.step === step.key) };
  }
  return base;
}

function frameData(step: DefinedStep, source: RunSource): FrameData {
  const run = runOf(source, step.key);
  if (step.kind === "loop") {
    const until = `until ${step.until.step} ${step.until.outcome}`;
    const detail = run.iterations ? `iter ${run.iterations} · ${until}` : until;
    return { name: step.key, kind: "loop", state: run.state, caption: "loop", detail, ...ports() };
  }
  const items = step.key === source.itemStep ? source.items : [];
  const done = items.filter((item) => item.state === "succeeded").length;
  const workflow = step.kind === "per_item" ? step.workflow : "";
  return {
    name: step.key,
    kind: "per_item",
    state: run.state,
    caption: "per item",
    detail: items.length ? `${workflow} · ${done}/${items.length}` : `${workflow} · no items yet`,
    inputs: [{ id: IN, label: step.kind === "per_item" ? step.items : "" }],
    outputs: [{ id: OUT, label: "" }],
  };
}

const TERMINAL_ITEM: StepState[] = ["succeeded", "cancelled"];

const ITEM_STATE: Record<ItemView["state"], StepState> = {
  pending: "pending",
  queued: "pending",
  running: "running",
  interrupted: "interrupted",
  blocked: "blocked",
  succeeded: "succeeded",
  cancelled: "cancelled",
};

function itemNode(key: string, position: Point, step: DefinedStep, source: RunSource): GraphNode {
  const item = source.items.find((candidate) => candidate.key === key);
  const state = item ? ITEM_STATE[item.state] : "pending";
  const data: StepData = {
    key: `${ITEM_PREFIX}${key}`,
    name: TERMINAL_ITEM.includes(state) || !item?.step ? state : item.step,
    kind: "item",
    state,
    itemKey: key,
    workflow: step.kind === "per_item" ? step.workflow : undefined,
    currentStep: item?.step ?? undefined,
    iteration: item?.iteration ?? undefined,
    attempts: item?.attempt ?? 0,
    needs: item?.needs ?? [],
    waitsFor: item?.waitsFor ?? [],
    mergedAs: item?.mergedAs ?? undefined,
    summary: itemSummary(item),
    executions: [],
    published: [],
    ...ports(),
  };
  return { id: data.key, type: "step", position, parentId: step.key, data };
}

function itemSummary(item: ItemView | undefined) {
  if (!item) return "child run";
  if (item.mergedAs) return `merged ${item.mergedAs.slice(0, 7)}`;
  if (item.waitsFor.length) return `waits for ${item.waitsFor.join(", ")}`;
  if (item.iteration) return `iter ${item.iteration} · attempt ${item.attempt ?? 1}`;
  return "child run";
}
