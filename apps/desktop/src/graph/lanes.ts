import type { InternalNode, Node } from "@xyflow/react";
import { LOOP_LANE } from "../engine/arrange";

type Lookup = Map<string, InternalNode<Node>>;
type Box = { x: number; y: number; width: number; height: number };

const BELOW = 40;

export function loopLane(lookup: Lookup, frame: string): number | undefined {
  const box = boxOf(lookup.get(frame));
  return box ? box.y + box.height - LOOP_LANE / 2 : undefined;
}

export function wrapLane(lookup: Lookup, source: string, target: string): number | undefined {
  const from = boxOf(outermost(lookup, source));
  const to = boxOf(outermost(lookup, target));
  if (!from || !to) return undefined;
  if (to.y <= from.y + from.height) return Math.max(from.y + from.height, to.y + to.height) + BELOW;
  const tops = [...lookup.values()].filter((node) => !node.parentId).map(boxOf).filter((box): box is Box => Boolean(box));
  const above = tops.filter((box) => box.y + box.height <= to.y + 1).map((box) => box.y + box.height);
  const ceiling = above.length ? Math.max(...above) : from.y + from.height;
  const floor = Math.min(to.y, ...tops.filter((box) => box.y >= ceiling).map((box) => box.y));
  return (ceiling + floor) / 2;
}

function outermost(lookup: Lookup, id: string) {
  let node = lookup.get(id);
  while (node?.parentId) node = lookup.get(node.parentId);
  return node;
}

function boxOf(node: InternalNode<Node> | undefined): Box | undefined {
  if (!node) return undefined;
  const { x, y } = node.internals.positionAbsolute;
  return { x, y, width: node.measured.width ?? node.width ?? 0, height: node.measured.height ?? node.height ?? 0 };
}
