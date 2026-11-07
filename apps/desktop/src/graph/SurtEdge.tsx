import { BaseEdge, EdgeLabelRenderer, getBezierPath, type EdgeProps } from "@xyflow/react";
import type { GraphEdge } from "../data/types";

type EdgeData = { state: GraphEdge["state"]; label?: string };

export function SurtEdge(props: EdgeProps) {
  const { id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition } = props;
  const data = props.data as EdgeData;
  const [path, labelX, labelY] = getBezierPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition });
  const active = data.state === "active";
  const gradientId = `ignite-${id.replace(/[^a-zA-Z0-9]/g, "")}`;

  return (
    <>
      {active ? (
        <defs>
          <linearGradient id={gradientId} gradientUnits="userSpaceOnUse" x1={sourceX} y1={sourceY} x2={targetX} y2={targetY}>
            <stop offset="0%" stopColor="var(--surt-lava)" />
            <stop offset="100%" stopColor="var(--surt-brasa)" />
          </linearGradient>
        </defs>
      ) : null}
      <BaseEdge id={id} path={path} className={`edge edge--${data.state}`} style={active ? { stroke: `url(#${gradientId})` } : undefined} />
      {active ? <FlowOverlay path={path} /> : null}
      {data.label ? (
        <EdgeLabelRenderer>
          <span className="edge__label mono" style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}>
            {data.label}
          </span>
        </EdgeLabelRenderer>
      ) : null}
    </>
  );
}

function FlowOverlay({ path }: { path: string }) {
  return (
    <>
      <path d={path} className="edge__flow" fill="none" />
      <circle r="3" className="edge__spark">
        <animateMotion dur="1.8s" repeatCount="indefinite" path={path} />
      </circle>
    </>
  );
}
