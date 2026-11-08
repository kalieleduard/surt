import { useEffect, useMemo, useState } from "react";
import {
  Background,
  BackgroundVariant,
  MiniMap,
  ReactFlow,
  useNodesState,
  useReactFlow,
  type Edge,
  type FitViewOptions,
  type Node,
  type NodeMouseHandler,
  type OnNodeDrag,
} from "@xyflow/react";
import "@xyflow/react/dist/base.css";
import type { RunGraph, StepData } from "../data/types";
import { useDefinitions } from "../editor/DefinitionsContext";
import { useEngine } from "../engine/EngineContext";
import { NODE_BOX } from "../engine/arrange";
import { align, type Alignment, type Rect } from "./alignment";
import { FrameNode } from "./FrameNode";
import { layoutKey, loadPlacements, storePlacements, type Placements } from "./layoutStore";
import { useRunView } from "./RunViewContext";
import { CanvasNotice } from "./CanvasNotice";
import { SelectionBar } from "./SelectionBar";
import { StepNode } from "./StepNode";
import { SurtEdge } from "./SurtEdge";
import "./graph.css";

const NODE_TYPES = { step: StepNode, frame: FrameNode };
const EDGE_TYPES = { surt: SurtEdge };
const PAN_WITH_MIDDLE_OR_RIGHT_BUTTON = [1, 2];
const SNAP_GRID: [number, number] = [18, 18];

function toFlowNodes(graph: RunGraph | null, selectedId: string, placements: Placements): Node[] {
  if (!graph) return [];
  return graph.nodes.map((node) => ({
    id: node.id,
    type: node.type,
    position: placements[node.id] ?? node.position,
    parentId: node.parentId,
    data: node.data,
    width: node.width,
    height: node.height,
    style: node.width ? { width: node.width, height: node.height } : undefined,
    selected: node.id === selectedId,
    zIndex: node.type === "frame" ? -1 : 1,
  }));
}

function toFlowEdges(graph: RunGraph | null): Edge[] {
  if (!graph) return [];
  return graph.edges.map((edge) => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
    sourceHandle: edge.sourceHandle,
    targetHandle: edge.targetHandle,
    type: "surt",
    zIndex: 0,
    data: { state: edge.state, label: edge.label },
  }));
}

function usePlacements(key: string | null, resetToken: number) {
  const [state, setState] = useState<{ key: string | null; placements: Placements }>(() => ({
    key,
    placements: key ? loadPlacements(key) : {},
  }));
  const current = state.key === key ? state.placements : key ? loadPlacements(key) : {};

  useEffect(() => {
    if (state.key !== key) setState({ key, placements: key ? loadPlacements(key) : {} });
  }, [key, state.key]);

  useEffect(() => {
    if (!resetToken || !key) return;
    storePlacements(key, {});
    setState({ key, placements: {} });
  }, [resetToken]);

  const place = (moved: Placements) => {
    if (!key) return;
    const next = { ...current, ...moved };
    storePlacements(key, next);
    setState({ key, placements: next });
  };

  return { placements: current, place };
}

export function RunCanvas({ padding }: { padding: FitViewOptions["padding"] }) {
  const { graph, selectedId, select, drill, tool, snap, resetToken } = useRunView();
  const { project } = useEngine();
  const { edit } = useDefinitions();
  const key = graph && project ? layoutKey(project.root, graph.item ? `${graph.workflow}#item` : graph.workflow) : null;
  const { placements, place } = usePlacements(key, resetToken);
  const initialNodes = useMemo(() => toFlowNodes(graph, selectedId, placements), [graph, placements]);
  const edges = useMemo(() => toFlowEdges(graph), [graph]);
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const flow = useReactFlow();
  const panning = tool === "pan";
  const selection = nodes.filter((node) => node.selected);

  useEffect(() => {
    setNodes((current) => {
      const chosen = new Set(current.filter((node) => node.selected).map((node) => node.id));
      if (!chosen.size) return initialNodes;
      return initialNodes.map((node) => ({ ...node, selected: chosen.has(node.id) }));
    });
  }, [initialNodes]);

  useEffect(() => {
    if (graph) requestAnimationFrame(() => flow.fitView({ padding, duration: 250 }));
  }, [graph?.id]);

  useEffect(() => {
    setNodes((current) => current.map((node) => ({ ...node, selected: node.id === selectedId })));
  }, [selectedId]);

  useEffect(() => {
    if (resetToken) requestAnimationFrame(() => flow.fitView({ padding, duration: 250 }));
  }, [resetToken]);

  const onNodeClick: NodeMouseHandler = (event, node) => {
    if (event.ctrlKey || event.metaKey || event.shiftKey) return;
    if (node.type === "step") select(node.id);
  };

  const onNodeDoubleClick: NodeMouseHandler = (_, node) => {
    const data = node.data as StepData;
    if (data.kind === "item" && data.itemKey) drill(data.itemKey);
    else if (data.agent) edit({ kind: "agent", id: data.agent });
  };

  const onNodeDragStop: OnNodeDrag = (_, __, dragged) => {
    place(Object.fromEntries(dragged.map((node) => [node.id, node.position])));
  };

  const arrangeSelection = (alignment: Alignment) => {
    const chosen = new Set(selection.map((node) => node.id));
    const roots = selection.filter((node) => !hasChosenAncestor(node, chosen, nodes));
    const rects = roots.map((node) => rectOf(flow, node));
    const moved = align(rects, alignment);
    const relative: Placements = {};
    for (const node of roots) {
      const target = moved.get(node.id);
      if (!target) continue;
      const origin = node.parentId ? absolute(flow, node.parentId) : { x: 0, y: 0 };
      relative[node.id] = { x: Math.round(target.x - origin.x), y: Math.round(target.y - origin.y) };
    }
    setNodes((current) => current.map((node) => (relative[node.id] ? { ...node, position: relative[node.id] } : node)));
    place(relative);
  };

  return (
    <div className={`run-canvas${panning ? " is-panning" : ""}`}>
      <CanvasNotice />
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={NODE_TYPES}
        edgeTypes={EDGE_TYPES}
        onNodesChange={onNodesChange}
        onNodeClick={onNodeClick}
        onNodeDoubleClick={onNodeDoubleClick}
        onNodeDragStop={onNodeDragStop}
        fitView
        fitViewOptions={{ padding }}
        minZoom={0.3}
        maxZoom={1.75}
        nodesConnectable={false}
        nodesDraggable={!panning}
        elementsSelectable={!panning}
        panOnDrag={panning ? true : PAN_WITH_MIDDLE_OR_RIGHT_BUTTON}
        selectionOnDrag={!panning}
        snapToGrid={snap}
        snapGrid={SNAP_GRID}
        multiSelectionKeyCode={["Control", "Meta", "Shift"]}
        selectionKeyCode={null}
        zoomOnDoubleClick={false}
      >
        <Background variant={BackgroundVariant.Dots} gap={SNAP_GRID[0]} size={1.4} />
        <MiniMap className="run-minimap" position="bottom-right" pannable zoomable nodeStrokeWidth={2} />
      </ReactFlow>
      {selection.length > 1 ? <SelectionBar count={selection.length} onArrange={arrangeSelection} /> : null}
    </div>
  );
}

type Flow = ReturnType<typeof useReactFlow>;

function absolute(flow: Flow, id: string) {
  return flow.getInternalNode(id)?.internals.positionAbsolute ?? { x: 0, y: 0 };
}

function rectOf(flow: Flow, node: Node): Rect {
  const internal = flow.getInternalNode(node.id);
  const position = internal?.internals.positionAbsolute ?? node.position;
  return {
    id: node.id,
    x: position.x,
    y: position.y,
    width: internal?.measured.width ?? node.width ?? NODE_BOX.width,
    height: internal?.measured.height ?? node.height ?? NODE_BOX.height,
  };
}

function hasChosenAncestor(node: Node, chosen: Set<string>, nodes: Node[]): boolean {
  let parent = node.parentId;
  while (parent) {
    if (chosen.has(parent)) return true;
    parent = nodes.find((candidate) => candidate.id === parent)?.parentId;
  }
  return false;
}
