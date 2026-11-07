import { Background, BackgroundVariant, useStore } from "@xyflow/react";
import type { CanvasBackground as Variant } from "../prototype/options";

const DIAGONAL_GAP = 28;

export function CanvasBackground({ variant }: { variant: Variant }) {
  if (variant === "dots") {
    return <Background variant={BackgroundVariant.Dots} gap={18} size={1.4} color="var(--canvas-mark)" />;
  }
  if (variant === "grid") {
    return <Background variant={BackgroundVariant.Lines} gap={24} lineWidth={1} color="var(--canvas-mark)" />;
  }
  return <DiagonalBackground />;
}

function DiagonalBackground() {
  const [x, y, zoom] = useStore((state) => state.transform);
  const gap = DIAGONAL_GAP * zoom;
  return (
    <div
      className="canvas-diagonal"
      style={{
        backgroundSize: `${gap}px ${gap}px`,
        backgroundPosition: `${x}px ${y}px`,
      }}
    />
  );
}
