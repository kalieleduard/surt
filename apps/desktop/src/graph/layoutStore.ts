import type { Point } from "../engine/arrange";

export type Placement = Point & { width?: number; height?: number };
export type Placements = Record<string, Placement>;

const PREFIX = "surt-layout";

export function layoutKey(root: string, workflow: string) {
  return `${PREFIX}:${root}:${workflow}`;
}

export function loadPlacements(key: string): Placements {
  try {
    const stored = localStorage.getItem(key);
    return stored ? (JSON.parse(stored) as Placements) : {};
  } catch {
    return {};
  }
}

export function storePlacements(key: string, placements: Placements) {
  try {
    if (Object.keys(placements).length) localStorage.setItem(key, JSON.stringify(placements));
    else localStorage.removeItem(key);
  } catch {
    return;
  }
}

export function merged(current: Placements, moved: Placements): Placements {
  const next = { ...current };
  for (const [id, placement] of Object.entries(moved)) next[id] = { ...current[id], ...placement };
  return next;
}
