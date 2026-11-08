import { EdgeLabelRenderer, useStore, type EdgeProps } from "@xyflow/react";
import type { GraphEdge } from "../data/types";
import { loopLane, wrapLane } from "./lanes";
import { route } from "./route";

const LOOP_STUB = 8;

export type EdgeData = { state: GraphEdge["state"]; label?: string; frame?: string; related?: boolean };

export function SurtEdge(props: EdgeProps) {
  const { id, source, target, sourceX, sourceY, targetX, targetY } = props;
  const data = props.data as EdgeData;
  const backward = targetX <= sourceX + 2;
  const lane = useStore((store) => {
    if (data.frame) return loopLane(store.nodeLookup, data.frame);
    return backward ? wrapLane(store.nodeLookup, source, target) : undefined;
  });
  const { path, label, arrow } = route({ x: sourceX, y: sourceY }, { x: targetX, y: targetY }, lane, data.frame ? LOOP_STUB : undefined);
  const active = data.state === "active";
  const className = `edge edge--${data.state}${data.related ? " is-related" : ""}`;

  return (
    <g className={className}>
      <path d={path} className="edge__hit" fill="none" />
      <path id={id} d={path} className="edge__line" fill="none" />
      {active ? <FlowOverlay path={path} /> : null}
      <path d={arrow} className="edge__arrow" />
      {data.label ? (
        <EdgeLabelRenderer>
          <span className={`edge__label mono edge__label--${data.state}`} style={{ transform: `translate(-50%, -50%) translate(${label.x}px, ${label.y}px)` }}>
            {data.label}
          </span>
        </EdgeLabelRenderer>
      ) : null}
    </g>
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
