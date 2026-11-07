import { BaseEdge, EdgeLabelRenderer, getBezierPath, getSmoothStepPath, type EdgeProps } from "@xyflow/react";
import type { GraphEdge } from "../data/types";
import { useChoice } from "../prototype/ChoiceContext";

type EdgeData = { state: GraphEdge["state"]; label?: string };

export function SurtEdge(props: EdgeProps) {
  const { id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition } = props;
  const data = props.data as EdgeData;
  const { edge, node } = useChoice();
  const geometry = { sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition };
  const [path, labelX, labelY] =
    edge === "orthogonal" ? getSmoothStepPath({ ...geometry, borderRadius: 6, offset: 24 }) : getBezierPath(geometry);
  const active = data.state === "active";
  const gradientId = `ignite-${id.replace(/[^a-zA-Z0-9]/g, "")}`;
  const stroke = active ? `url(#${gradientId})` : undefined;
  const showLabel = data.label && node !== "card";

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
      <BaseEdge id={id} path={path} className={`edge edge--${data.state} edge-style--${edge}`} style={stroke ? { stroke } : undefined} />
      {active && edge === "flow" ? <FlowOverlay path={path} /> : null}
      {showLabel ? (
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
