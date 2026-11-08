import { list, table, text } from "./contracts";
import type { TomlTable } from "./toml";
import type { Condition, DefinedStep } from "./types";

export function definedSteps(workflow: TomlTable): DefinedStep[] {
  return Object.entries(table(workflow.steps)).flatMap(([key, value]): DefinedStep[] => {
    const raw = table(value);
    const base = { key, needs: list(raw.needs), when: condition(raw.when) };
    const kind = text(raw.kind);
    if (kind === "agent") return [{ ...base, kind, agent: text(raw.agent) ?? "" }];
    if (kind === "approval" || kind === "input") return [{ ...base, kind, artifact: text(raw.artifact) ?? "" }];
    if (kind === "operation") return [{ ...base, kind, operation: text(raw.run) ?? "" }];
    if (kind === "loop") return [{ ...base, kind, body: list(raw.body), until: condition(raw.until) ?? { step: "", outcome: "" } }];
    if (kind === "per_item") return [{ ...base, kind, items: text(raw.items) ?? "", workflow: text(raw.workflow) ?? "" }];
    return [];
  });
}

function condition(value: TomlTable[string] | undefined): Condition | null {
  const raw = table(value);
  const step = text(raw.step);
  const outcome = text(raw.outcome);
  return step && outcome ? { step, outcome } : null;
}
