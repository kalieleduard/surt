import type { DefinedStep } from "./types";

export type DefinitionChanges = { added: string[]; removed: string[]; changed: string[] };

export function definitionChanges(frozen: DefinedStep[], current: DefinedStep[]): DefinitionChanges {
  const before = new Map(frozen.map((step) => [step.key, step]));
  const after = new Map(current.map((step) => [step.key, step]));
  return {
    added: current.filter((step) => !before.has(step.key)).map((step) => step.key),
    removed: frozen.filter((step) => !after.has(step.key)).map((step) => step.key),
    changed: current.filter((step) => before.has(step.key) && !same(before.get(step.key), step)).map((step) => step.key),
  };
}

export function hasChanges(changes: DefinitionChanges): boolean {
  return changes.added.length + changes.removed.length + changes.changed.length > 0;
}

function same(left: DefinedStep | undefined, right: DefinedStep): boolean {
  return JSON.stringify(normalized(left)) === JSON.stringify(normalized(right));
}

function normalized(step: DefinedStep | undefined) {
  if (!step) return null;
  return Object.fromEntries(Object.entries(step).sort(([a], [b]) => a.localeCompare(b)));
}
