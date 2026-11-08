import { useEffect, useState } from "react";
import { loadPlacements, merged, storePlacements, type Placements } from "./layoutStore";

export function usePlacements(key: string | null, resetToken: number) {
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
    const next = merged(current, moved);
    storePlacements(key, next);
    setState({ key, placements: next });
  };

  return { placements: current, place };
}
