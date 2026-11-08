import type { GraphEdge, GraphNode, StepData, StepState } from "../data/types";
import { layers } from "./arrange";
import { IN, ITEM_PREFIX, OUT, needsOf, type RunSource } from "./runNodes";
import type { DefinedStep } from "./types";

type Handle = { node: string; handle: string };
type States = Map<string, StepState>;

export const ENTER = "enter";
export const LEAVE = "leave";

export function runEdges(source: RunSource, nodes: GraphNode[]): GraphEdge[] {
  const states: States = new Map(nodes.map((node) => [node.id, (node.data as StepData).state]));
  const edges: GraphEdge[] = [];
  for (const step of source.steps.values()) {
    for (const need of step.needs) edges.push(link(exit(need), entry(step.key), flow(states.get(step.key)), condition(step, need)));
    if (step.kind === "loop") edges.push(...loopEdges(source, step, states));
  }
  for (const item of source.items) {
    for (const need of item.needs) {
      edges.push(link(exit(`${ITEM_PREFIX}${need}`), entry(`${ITEM_PREFIX}${item.key}`), flow(states.get(`${ITEM_PREFIX}${item.key}`))));
    }
  }
  return edges.filter((edge) => states.has(edge.source) && states.has(edge.target));
}

function loopEdges(source: RunSource, loop: DefinedStep & { kind: "loop" }, states: States): GraphEdge[] {
  const first = layers(loop.body, needsOf(source))[0] ?? [];
  const inside = new Set(loop.body);
  const needed = new Set(loop.body.flatMap((member) => needsOf(source)(member).filter((need) => inside.has(need))));
  const tails = loop.body.filter((member) => !needed.has(member));
  const iterations = source.runs.get(loop.key)?.iterations ?? 0;
  const edges = first.map((member) => link({ node: loop.key, handle: `${ENTER}:${IN}` }, entry(member), flow(states.get(member))));
  const until = loop.until.step;
  if (inside.has(until)) edges.push(link(exit(until), { node: loop.key, handle: `${LEAVE}:${OUT}` }, reached(states.get(loop.key))));
  if (first[0]) {
    tails.forEach((tail, index) => {
      const back = link(exit(tail), entry(first[0]), "back", index === 0 ? iterationLabel(iterations) : undefined);
      edges.push({ ...back, id: index === 0 ? `${loop.key}:back` : `${loop.key}:back:${tail}`, frame: loop.key });
    });
  }
  return edges;
}

function entry(key: string): Handle {
  return { node: key, handle: `in:${IN}` };
}

function exit(key: string): Handle {
  return { node: key, handle: `out:${OUT}` };
}

function condition(step: DefinedStep, need: string): string | undefined {
  if (!step.when) return undefined;
  return step.when.step === need ? step.when.outcome : `${step.when.step} = ${step.when.outcome}`;
}

function iterationLabel(iterations: number) {
  return iterations > 1 ? `↺ iter ${iterations}` : "↺ repeat";
}

function link(from: Handle, to: Handle, state: GraphEdge["state"], label?: string): GraphEdge {
  return {
    id: `${from.node}->${to.node}`,
    source: from.node,
    target: to.node,
    sourceHandle: from.handle,
    targetHandle: to.handle,
    label,
    state,
  };
}

function flow(target: StepState | undefined): GraphEdge["state"] {
  if (!target || target === "pending" || target === "skipped") return "idle";
  if (target === "running" || target === "waiting") return "active";
  return "done";
}

function reached(source: StepState | undefined): GraphEdge["state"] {
  return source === "succeeded" ? "done" : "idle";
}
