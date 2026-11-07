import { useId } from "react";
import {
  CANVAS_EDGE_X,
  CODE_PORTS,
  EDGES,
  HOT_SPAN,
  INSPECTOR_IDLE_PORTS,
  INSPECTOR_PORTS,
  NODES,
  curvePath,
  portPoint,
  type GraphNode,
  type PortKind,
  type PortSide,
  type Point,
} from "./referenceGraph";

type PortMark = Point & { kind: PortKind; side: PortSide; key: string };

function nodePorts(name: string, node: GraphNode): PortMark[] {
  return node.rows.flatMap((row, index) => {
    const marks: PortMark[] = [];
    if (row.inPort) marks.push({ ...portPoint(node, index, "in"), kind: row.inPort, side: "in", key: `${name}-in-${index}` });
    if (row.outPort) marks.push({ ...portPoint(node, index, "out"), kind: row.outPort, side: "out", key: `${name}-out-${index}` });
    return marks;
  });
}

const INSPECTOR_MARKS: PortMark[] = [
  ...Object.values(INSPECTOR_PORTS).map((y) => ({ x: CANVAS_EDGE_X, y, kind: "linked" as const, side: "in" as const, key: `insp-${y}` })),
  ...INSPECTOR_IDLE_PORTS.map((y) => ({ x: CANVAS_EDGE_X, y, kind: "idle" as const, side: "in" as const, key: `insp-${y}` })),
];

const CODE_MARKS: PortMark[] = [
  { ...CODE_PORTS.input, kind: "linked", side: "in", key: "code-in" },
  { ...CODE_PORTS.stub, kind: "idle", side: "in", key: "code-stub" },
  { ...CODE_PORTS.output, kind: "linked", side: "out", key: "code-out" },
];

const PORT_MARKS: PortMark[] = [
  ...Object.entries(NODES).flatMap(([name, node]) => nodePorts(name, node)),
  ...CODE_MARKS,
  ...INSPECTOR_MARKS,
];

function PortDot({ mark }: { mark: PortMark }) {
  if (mark.kind === "idle") return <circle className="ref-port-idle" cx={mark.x} cy={mark.y} r={1.7} />;
  if (mark.kind === "linked") return <circle className="ref-port-linked" cx={mark.x} cy={mark.y} r={2.5} />;
  return (
    <g>
      <circle className="ref-port-ring" data-side={mark.side} cx={mark.x} cy={mark.y} r={4.2} />
      <circle className="ref-port-linked" cx={mark.x} cy={mark.y} r={1.9} />
    </g>
  );
}

export function ReferenceEdges() {
  const gradientId = `ref-ignite-${useId().replace(/[^a-zA-Z0-9_-]/g, "")}`;
  return (
    <svg className="ref-edges" aria-hidden="true">
      <defs>
        <linearGradient id={gradientId} gradientUnits="userSpaceOnUse" x1={HOT_SPAN.from} y1={0} x2={HOT_SPAN.to} y2={0}>
          <stop className="ref-stop-lava" offset="0" />
          <stop className="ref-stop-brasa" offset="1" />
        </linearGradient>
      </defs>
      {EDGES.map((edge) => (
        <path
          key={edge.id}
          className={edge.hot ? "ref-edge ref-edge-hot" : "ref-edge ref-edge-cool"}
          style={edge.hot ? { stroke: `url(#${gradientId})` } : undefined}
          d={curvePath(edge.from, edge.to)}
        />
      ))}
      {PORT_MARKS.map((mark) => (
        <PortDot key={mark.key} mark={mark} />
      ))}
    </svg>
  );
}
