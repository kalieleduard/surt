import type { GraphEdge, GraphNode, StepData, StepState } from "../data/types";
import { layers } from "./arrange";
import { IN, ITEM_PREFIX, OUT, needsOf, type RunSource } from "./runNodes";

type Handle = { node: string; handle: string };

const MAX_LOOP_DEPTH = 8;

export function runEdges(source: RunSource, nodes: GraphNode[]): GraphEdge[] {
  const states = new Map(nodes.map((node) => [node.id, (node.data as StepData).state]));
  const edges: GraphEdge[] = [];
  for (const step of source.steps.values()) {
    for (const need of step.needs) edges.push(link(exit(source, need), entry(source, step.key), states));
    if (step.kind === "loop") edges.push(backEdge(source, step.key, states));
  }
  for (const item of source.items) {
    for (const need of item.needs) {
      const from = { node: `${ITEM_PREFIX}${need}`, handle: `out:${OUT}` };
      edges.push(link(from, { node: `${ITEM_PREFIX}${item.key}`, handle: `in:${IN}` }, states));
    }
  }
  return edges.filter((edge) => states.has(edge.source) && states.has(edge.target));
}

function entry(source: RunSource, key: string, depth = 0): Handle {
  const step = source.steps.get(key);
  if (step?.kind === "loop" && step.body.length && depth < MAX_LOOP_DEPTH) {
    return entry(source, layers(step.body, needsOf(source))[0][0], depth + 1);
  }
  return { node: key, handle: `in:${IN}` };
}

function exit(source: RunSource, key: string, depth = 0): Handle {
  const step = source.steps.get(key);
  if (step?.kind === "loop" && depth < MAX_LOOP_DEPTH) return exit(source, step.until.step, depth + 1);
  return { node: key, handle: `out:${OUT}` };
}

function backEdge(source: RunSource, key: string, states: Map<string, StepState>): GraphEdge {
  const iterations = source.runs.get(key)?.iterations ?? 0;
  const edge = link(exit(source, key), entry(source, key), states, iterations > 1 ? `iter ${iterations}` : undefined);
  return { ...edge, id: `${key}:back`, state: "back" };
}

function link(from: Handle, to: Handle, states: Map<string, StepState>, label?: string): GraphEdge {
  return {
    id: `${from.node}->${to.node}`,
    source: from.node,
    target: to.node,
    sourceHandle: from.handle,
    targetHandle: to.handle,
    label,
    state: flow(states.get(to.node)),
  };
}

function flow(target: StepState | undefined): GraphEdge["state"] {
  if (!target || target === "pending" || target === "skipped") return "idle";
  if (target === "running" || target === "waiting") return "active";
  return "done";
}
