import type { Point } from "../engine/arrange";

export type Rect = { id: string; x: number; y: number; width: number; height: number };

export type Alignment =
  | "left"
  | "center"
  | "right"
  | "top"
  | "middle"
  | "bottom"
  | "distribute-x"
  | "distribute-y"
  | "row"
  | "column";

export const UNIFORM_GAP = 48;

type Axis = { start: "x" | "y"; size: "width" | "height" };

const HORIZONTAL: Axis = { start: "x", size: "width" };
const VERTICAL: Axis = { start: "y", size: "height" };

export function align(rects: Rect[], alignment: Alignment, gap = UNIFORM_GAP): Map<string, Point> {
  if (rects.length < 2) return new Map(rects.map((rect) => [rect.id, { x: rect.x, y: rect.y }]));
  switch (alignment) {
    case "left":
      return edge(rects, HORIZONTAL, 0);
    case "center":
      return edge(rects, HORIZONTAL, 0.5);
    case "right":
      return edge(rects, HORIZONTAL, 1);
    case "top":
      return edge(rects, VERTICAL, 0);
    case "middle":
      return edge(rects, VERTICAL, 0.5);
    case "bottom":
      return edge(rects, VERTICAL, 1);
    case "distribute-x":
      return distribute(rects, HORIZONTAL);
    case "distribute-y":
      return distribute(rects, VERTICAL);
    case "row":
      return line(rects, HORIZONTAL, VERTICAL, gap);
    case "column":
      return line(rects, VERTICAL, HORIZONTAL, gap);
  }
}

function edge(rects: Rect[], axis: Axis, anchor: number): Map<string, Point> {
  const target = anchor === 0.5 ? mean(rects, axis) : extreme(rects, axis, anchor);
  return new Map(
    rects.map((rect) => [rect.id, with_(rect, axis, target - rect[axis.size] * anchor)]),
  );
}

function extreme(rects: Rect[], axis: Axis, anchor: number): number {
  const edges = rects.map((rect) => rect[axis.start] + rect[axis.size] * anchor);
  return anchor === 0 ? Math.min(...edges) : Math.max(...edges);
}

function mean(rects: Rect[], axis: Axis): number {
  const low = Math.min(...rects.map((rect) => rect[axis.start]));
  const high = Math.max(...rects.map((rect) => rect[axis.start] + rect[axis.size]));
  return (low + high) / 2;
}

function distribute(rects: Rect[], axis: Axis): Map<string, Point> {
  const sorted = sortAlong(rects, axis);
  const first = sorted[0];
  const last = sorted[sorted.length - 1];
  const span = last[axis.start] + last[axis.size] - first[axis.start];
  const occupied = sorted.reduce((sum, rect) => sum + rect[axis.size], 0);
  const gap = (span - occupied) / (sorted.length - 1);
  return pack(sorted, axis, first[axis.start], gap);
}

function line(rects: Rect[], axis: Axis, cross: Axis, gap: number): Map<string, Point> {
  const sorted = sortAlong(rects, axis);
  const center = mean(rects, cross);
  const packed = pack(sorted, axis, sorted[0][axis.start], gap);
  for (const rect of sorted) {
    const point = packed.get(rect.id);
    if (point) point[cross.start] = center - rect[cross.size] / 2;
  }
  return packed;
}

function pack(sorted: Rect[], axis: Axis, start: number, gap: number): Map<string, Point> {
  const positions = new Map<string, Point>();
  let cursor = start;
  for (const rect of sorted) {
    positions.set(rect.id, with_(rect, axis, cursor));
    cursor += rect[axis.size] + gap;
  }
  return positions;
}

function sortAlong(rects: Rect[], axis: Axis): Rect[] {
  return [...rects].sort((left, right) => left[axis.start] - right[axis.start] || left.id.localeCompare(right.id));
}

function with_(rect: Rect, axis: Axis, value: number): Point {
  const point = { x: rect.x, y: rect.y };
  point[axis.start] = value;
  return point;
}
