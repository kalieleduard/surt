import { useEffect, useMemo } from "react";
import {
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
import { useChoice } from "../prototype/ChoiceContext";
import { CanvasBackground } from "./CanvasBackground";
import { FrameNode } from "./FrameNode";
import { useRunView } from "./RunViewContext";
import { StepNode } from "./StepNode";
import { SurtEdge } from "./SurtEdge";
import "./graph.css";

const NODE_TYPES = { step: StepNode, frame: FrameNode };
const EDGE_TYPES = { surt: SurtEdge };

function toFlowNodes(graph: RunGraph, selectedId: string): Node[] {
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

function toFlowEdges(graph: RunGraph): Edge[] {
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

type RunCanvasProps = {
  padding?: FitViewOptions["padding"];
  focus?: string[];
  minimap?: boolean;
};

function fitOptions(graph: RunGraph, padding: FitViewOptions["padding"], focus?: string[]): FitViewOptions {
  const present = focus?.filter((id) => graph.nodes.some((node) => node.id === id)) ?? [];
  return present.length ? { padding, nodes: present.map((id) => ({ id })) } : { padding };
}

export function RunCanvas({ padding = 0.08, focus, minimap = false }: RunCanvasProps) {
  const { graph, selectedId, select, drill } = useRunView();
  const choice = useChoice();
  const initialNodes = useMemo(() => toFlowNodes(graph, selectedId), [graph]);
  const edges = useMemo(() => toFlowEdges(graph), [graph]);
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const flow = useReactFlow();
  const fit = fitOptions(graph, padding, focus);

  useEffect(() => {
    setNodes(initialNodes);
    requestAnimationFrame(() => flow.fitView({ ...fit, duration: 250 }));
  }, [initialNodes]);

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
    <div className={`run-canvas edge-mode--${choice.edge}`}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={NODE_TYPES}
        edgeTypes={EDGE_TYPES}
        onNodesChange={onNodesChange}
        onNodeClick={onNodeClick}
        onNodeDoubleClick={onNodeDoubleClick}
        fitView
        fitViewOptions={fit}
        minZoom={0.3}
        maxZoom={1.75}
        nodesConnectable={false}
        zoomOnDoubleClick={false}
      >
        <CanvasBackground variant={choice.background} />
        {minimap ? <MiniMap className="run-minimap" position="bottom-right" pannable zoomable nodeStrokeWidth={2} /> : null}
      </ReactFlow>
    </div>
  );
}
