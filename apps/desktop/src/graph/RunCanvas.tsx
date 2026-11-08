import { useEffect, useMemo } from "react";
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
} from "@xyflow/react";
import "@xyflow/react/dist/base.css";
import type { RunGraph, StepData } from "../data/types";
import { FrameNode } from "./FrameNode";
import { useRunView } from "./RunViewContext";
import { CanvasNotice } from "./CanvasNotice";
import { StepNode } from "./StepNode";
import { SurtEdge } from "./SurtEdge";
import "./graph.css";

const NODE_TYPES = { step: StepNode, frame: FrameNode };
const EDGE_TYPES = { surt: SurtEdge };
const PAN_WITH_MIDDLE_OR_RIGHT_BUTTON = [1, 2];

function toFlowNodes(graph: RunGraph | null, selectedId: string): Node[] {
  if (!graph) return [];
  return graph.nodes.map((node) => ({
    id: node.id,
    type: node.type,
    position: node.position,
    parentId: node.parentId,
    data: node.data,
    width: node.width,
    height: node.height,
    style: node.width ? { width: node.width, height: node.height } : undefined,
    selected: node.id === selectedId,
    selectable: node.type === "step",
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

export function RunCanvas({ padding }: { padding: FitViewOptions["padding"] }) {
  const { graph, selectedId, select, drill, tool } = useRunView();
  const initialNodes = useMemo(() => toFlowNodes(graph, selectedId), [graph]);
  const edges = useMemo(() => toFlowEdges(graph), [graph]);
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const flow = useReactFlow();
  const panning = tool === "pan";

  useEffect(() => {
    setNodes(initialNodes);
  }, [initialNodes]);

  useEffect(() => {
    if (graph) requestAnimationFrame(() => flow.fitView({ padding, duration: 250 }));
  }, [graph?.id]);

  useEffect(() => {
    setNodes((current) => current.map((node) => ({ ...node, selected: node.id === selectedId })));
  }, [selectedId]);

  const onNodeClick: NodeMouseHandler = (_, node) => {
    if (node.type === "step") select(node.id);
  };

  const onNodeDoubleClick: NodeMouseHandler = (_, node) => {
    const data = node.data as StepData;
    if (data.kind === "item" && data.itemKey) drill(data.itemKey);
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
        fitView
        fitViewOptions={{ padding }}
        minZoom={0.3}
        maxZoom={1.75}
        nodesConnectable={false}
        nodesDraggable={!panning}
        elementsSelectable={!panning}
        panOnDrag={panning ? true : PAN_WITH_MIDDLE_OR_RIGHT_BUTTON}
        selectionOnDrag={!panning}
        zoomOnDoubleClick={false}
      >
        <Background variant={BackgroundVariant.Dots} gap={18} size={1.4} />
        <MiniMap className="run-minimap" position="bottom-right" pannable zoomable nodeStrokeWidth={2} />
      </ReactFlow>
    </div>
  );
}
