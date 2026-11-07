import type { FrameData, GraphEdge, GraphNode, StepData } from "./types";

const NODE_WIDTH = 184;
const NODE_HEIGHT = 48;
const COLUMN_STEP = 220;
const FRAME_INSET = 16;
const FRAME_HEADER = 44;
const ROW_STEP = 120;

export function column(index: number) {
  return FRAME_INSET + index * COLUMN_STEP;
}

export function row(index: number) {
  return FRAME_HEADER + index * ROW_STEP;
}

export function frameSize(columns: number, rows: number): [number, number] {
  return [column(columns - 1) + NODE_WIDTH + FRAME_INSET, row(rows - 1) + NODE_HEIGHT + FRAME_INSET];
}

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
