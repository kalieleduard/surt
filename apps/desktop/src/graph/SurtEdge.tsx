import { useStore, type EdgeProps } from "@xyflow/react";
import type { GraphEdge } from "../data/types";
import { loopLane, wrapLane } from "./lanes";
import { route, type Point } from "./route";

const LOOP_STUB = 8;
const CHIP = { char: 6.35, padding: 8, height: 18 };

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
      {data.label ? <EdgeChip at={label} text={data.label} state={data.state} /> : null}
    </g>
  );
}

function EdgeChip({ at, text, state }: { at: Point; text: string; state: EdgeData["state"] }) {
  const width = Math.round(text.length * CHIP.char + CHIP.padding * 2);
  return (
    <g className={`edge__chip edge__chip--${state}`} transform={`translate(${Math.round(at.x - width / 2)} ${Math.round(at.y - CHIP.height / 2)})`}>
      <rect width={width} height={CHIP.height} rx={CHIP.height / 2} />
      <text x={width / 2} y={CHIP.height / 2} dominantBaseline="central" textAnchor="middle">
        {text}
      </text>
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
