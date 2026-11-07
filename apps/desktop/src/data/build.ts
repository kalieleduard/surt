import type { FrameData, GraphEdge, GraphNode, StepData } from "./types";

export function step(id: string, x: number, y: number, data: StepData, parentId?: string): GraphNode {
  return { id, type: "step", position: { x, y }, parentId, data };
}

export function frame(id: string, x: number, y: number, size: [number, number], data: FrameData): GraphNode {
  return { id, type: "frame", position: { x, y }, width: size[0], height: size[1], data };
}

export function link(
  source: string,
  output: string,
  target: string,
  input: string,
  state: GraphEdge["state"],
  label?: string,
): GraphEdge {
  return {
    id: `${source}.${output}->${target}.${input}`,
    source,
    target,
    sourceHandle: `out:${output}`,
    targetHandle: `in:${input}`,
    state,
    label,
  };
}
