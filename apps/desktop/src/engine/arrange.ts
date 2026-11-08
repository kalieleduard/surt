export type Box = { width: number; height: number };
export type Point = { x: number; y: number };
export type Arrangement = { positions: Map<string, Point>; box: Box };

export const NODE_BOX: Box = { width: 184, height: 48 };
export const FRAME_INSET = 16;
export const FRAME_HEADER = 44;
export const MIN_FRAME_CONTENT = 404;

const COLUMN_GAP = 36;
const STACK_GAP = 24;
const ROW_GAP = 72;
const MAX_STACK = 3;

type Column = { keys: string[]; width: number; height: number };

export function layers(keys: string[], needsOf: (key: string) => string[]): string[][] {
  const members = new Set(keys);
  const depth = new Map<string, number>();
  const visit = (key: string, trail: Set<string>): number => {
    const known = depth.get(key);
    if (known !== undefined) return known;
    if (trail.has(key)) return 0;
    trail.add(key);
    const parents = needsOf(key).filter((need) => members.has(need));
    const value = parents.length ? Math.max(...parents.map((parent) => visit(parent, trail))) + 1 : 0;
    trail.delete(key);
    depth.set(key, value);
    return value;
  };
  const grouped: string[][] = [];
  for (const key of keys) {
    const index = visit(key, new Set());
    (grouped[index] ??= []).push(key);
  }
  return grouped.filter(Boolean);
}

export function stacked(groups: string[][]): string[][] {
  return groups.flatMap((group) => {
    const chunks: string[][] = [];
    for (let start = 0; start < group.length; start += MAX_STACK) chunks.push(group.slice(start, start + MAX_STACK));
    return chunks;
  });
}

export function framed(box: Box): Box {
  return { width: Math.max(box.width, MIN_FRAME_CONTENT) + 2 * FRAME_INSET, height: box.height + FRAME_HEADER + FRAME_INSET };
}

export function arrange(groups: string[][], sizeOf: (key: string) => Box, maxWidth = Infinity): Arrangement {
  const columns = groups.map((keys) => column(keys, sizeOf));
  const positions = new Map<string, Point>();
  let top = 0;
  let width = 0;
  for (const row of wrap(columns, maxWidth)) {
    const height = Math.max(...row.map((entry) => entry.height));
    let left = 0;
    for (const entry of row) {
      place(entry, { x: left, y: top + (height - entry.height) / 2 }, sizeOf, positions);
      left += entry.width + COLUMN_GAP;
    }
    width = Math.max(width, left - COLUMN_GAP);
    top += height + ROW_GAP;
  }
  return { positions, box: { width, height: Math.max(0, top - ROW_GAP) } };
}

function column(keys: string[], sizeOf: (key: string) => Box): Column {
  const boxes = keys.map(sizeOf);
  return {
    keys,
    width: Math.max(...boxes.map((box) => box.width)),
    height: boxes.reduce((sum, box) => sum + box.height, 0) + STACK_GAP * (boxes.length - 1),
  };
}

function wrap(columns: Column[], maxWidth: number): Column[][] {
  const rows: Column[][] = [];
  let current: Column[] = [];
  let width = 0;
  for (const entry of columns) {
    const grown = width + (current.length ? COLUMN_GAP : 0) + entry.width;
    if (current.length && grown > maxWidth) {
      rows.push(current);
      current = [];
    }
    width = current.length ? width + COLUMN_GAP + entry.width : entry.width;
    current.push(entry);
  }
  if (current.length) rows.push(current);
  return rows;
}

function place(entry: Column, origin: Point, sizeOf: (key: string) => Box, positions: Map<string, Point>) {
  let y = origin.y;
  for (const key of entry.keys) {
    const box = sizeOf(key);
    positions.set(key, { x: origin.x + (entry.width - box.width) / 2, y });
    y += box.height + STACK_GAP;
  }
}
