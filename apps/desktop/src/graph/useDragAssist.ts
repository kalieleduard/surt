import { useRef } from "react";
import { useReactFlow, type InternalNode, type Node, type NodeChange, type NodePositionChange, type OnNodeDrag, type XYPosition } from "@xyflow/react";
import { useShiftRef } from "./useShiftKey";

const MAGNET = 10;

type Axis = "horizontal" | "vertical" | null;
type Moving = NodePositionChange & { position: XYPosition };

export function useDragAssist() {
  const flow = useReactFlow();
  const shift = useShiftRef();
  const starts = useRef(new Map<string, XYPosition>());

  const onNodeDragStart: OnNodeDrag = (_, __, nodes) => {
    starts.current = new Map(nodes.map((node) => [node.id, { ...node.position }]));
  };

  const onNodeDragStop = () => {
    requestAnimationFrame(() => {
      starts.current = new Map();
    });
  };

  const assist = (changes: NodeChange[]): NodeChange[] => {
    const dragged = (change: NodeChange): change is Moving => change.type === "position" && Boolean(change.position) && starts.current.has(change.id);
    const moving = changes.filter(dragged);
    if (!moving.length) return changes;
    const axis = shift.current ? lockedAxis(moving[0], starts.current.get(moving[0].id)) : null;
    return changes.map((change) => {
      if (!dragged(change)) return change;
      const start = starts.current.get(change.id);
      const position = { ...change.position };
      if (start && axis === "horizontal") position.y = start.y;
      if (start && axis === "vertical") position.x = start.x;
      if (moving.length === 1 && axis !== "horizontal") position.y = magnetY(flow, change.id, position.y);
      return { ...change, position };
    });
  };

  return { onNodeDragStart, onNodeDragStop, assist };
}

function lockedAxis(change: Moving, start: XYPosition | undefined): Axis {
  if (!start) return null;
  const dx = Math.abs(change.position.x - start.x);
  const dy = Math.abs(change.position.y - start.y);
  return dx >= dy ? "horizontal" : "vertical";
}

type Flow = ReturnType<typeof useReactFlow>;

function magnetY(flow: Flow, id: string, y: number): number {
  const node = flow.getInternalNode(id);
  if (!node) return y;
  const parentY = node.parentId ? (flow.getInternalNode(node.parentId)?.internals.positionAbsolute.y ?? 0) : 0;
  let best: { y: number; distance: number } | null = null;
  for (const edge of flow.getEdges()) {
    if ((edge.data as { frame?: string } | undefined)?.frame) continue;
    const outgoing = edge.source === id;
    if (!outgoing && edge.target !== id) continue;
    const other = flow.getInternalNode(outgoing ? edge.target : edge.source);
    if (!other || other.id === node.parentId || other.parentId === id) continue;
    const mine = handleOffset(node, outgoing ? "source" : "target", outgoing ? edge.sourceHandle : edge.targetHandle);
    const theirs = handleOffset(other, outgoing ? "target" : "source", outgoing ? edge.targetHandle : edge.sourceHandle);
    if (mine === null || theirs === null) continue;
    const aligned = other.internals.positionAbsolute.y + theirs - parentY - mine;
    const distance = Math.abs(aligned - y);
    if (distance <= MAGNET && (!best || distance < best.distance)) best = { y: aligned, distance };
  }
  return best?.y ?? y;
}

function handleOffset(node: InternalNode<Node>, type: "source" | "target", id: string | null | undefined): number | null {
  const bounds = node.internals.handleBounds?.[type]?.find((handle) => handle.id === id);
  return bounds ? bounds.y + bounds.height / 2 : null;
}
