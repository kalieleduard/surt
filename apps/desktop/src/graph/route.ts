export type Point = { x: number; y: number };
export type Route = { path: string; label: Point; arrow: string };

const STUB = 18;
const RADIUS = 9;
const ARROW = { length: 7, half: 4 };
const STRAIGHT = 1;

export function route(from: Point, to: Point, lane?: number, stub = STUB): Route {
  const points = to.x > from.x + 2 ? forward(from, to) : backward(from, to, lane ?? Math.max(from.y, to.y) + 48, stub);
  const tip = { x: to.x, y: to.y };
  const end = { x: tip.x - ARROW.length, y: tip.y };
  const body = [...points.slice(0, -1), end];
  return { path: rounded(body), label: labelPoint(body, lane !== undefined || to.x <= from.x + 2), arrow: arrowPath(tip) };
}

function forward(from: Point, to: Point): Point[] {
  if (Math.abs(to.y - from.y) < STRAIGHT) return [from, to];
  const middle = from.x + Math.max(STUB, (to.x - from.x) / 2);
  const turn = Math.min(middle, to.x - ARROW.length - 2);
  return [from, { x: turn, y: from.y }, { x: turn, y: to.y }, to];
}

function backward(from: Point, to: Point, lane: number, stub: number): Point[] {
  const out = from.x + stub;
  const back = to.x - stub - ARROW.length;
  return [from, { x: out, y: from.y }, { x: out, y: lane }, { x: back, y: lane }, { x: back, y: to.y }, to];
}

function rounded(points: Point[]): string {
  const clean = points.filter((point, index) => index === 0 || distance(point, points[index - 1]) > 0.5);
  if (clean.length < 3) return clean.map((point, index) => `${index ? "L" : "M"} ${point.x} ${point.y}`).join(" ");
  let path = `M ${clean[0].x} ${clean[0].y}`;
  for (let index = 1; index < clean.length - 1; index += 1) {
    const [before, corner, after] = [clean[index - 1], clean[index], clean[index + 1]];
    const radius = Math.min(RADIUS, distance(before, corner) / 2, distance(corner, after) / 2);
    const enter = toward(corner, before, radius);
    const leave = toward(corner, after, radius);
    path += ` L ${enter.x} ${enter.y} Q ${corner.x} ${corner.y} ${leave.x} ${leave.y}`;
  }
  const last = clean[clean.length - 1];
  return `${path} L ${last.x} ${last.y}`;
}

function labelPoint(points: Point[], onLane: boolean): Point {
  if (onLane && points.length >= 4) return midpoint(points[2], points[3]);
  const horizontal = points.slice(1).map((point, index) => [points[index], point] as const).filter(([a, b]) => Math.abs(a.y - b.y) < 0.5);
  const [a, b] = horizontal.at(-1) ?? [points[0], points[points.length - 1]];
  return midpoint(a, b);
}

function arrowPath(tip: Point): string {
  return `M ${tip.x - ARROW.length} ${tip.y - ARROW.half} L ${tip.x} ${tip.y} L ${tip.x - ARROW.length} ${tip.y + ARROW.half} Z`;
}

function toward(from: Point, to: Point, length: number): Point {
  const total = distance(from, to) || 1;
  return { x: from.x + ((to.x - from.x) * length) / total, y: from.y + ((to.y - from.y) * length) / total };
}

function distance(a: Point, b: Point) {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function midpoint(a: Point, b: Point): Point {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}
