import type { Condition, DefinedStep, GateSummary, ItemStateName, ItemView, ProjectView, RunDetail, SubjectView, TimelineEvent } from "./types";

export const PROJECT: ProjectView = { root: "C:/work/shop", name: "shop", maxAttempts: 3, maxIterations: 5 };

export function agent(key: string, needs: string[] = []): DefinedStep {
  return { key, needs, when: null, kind: "agent", agent: key };
}

export function approval(key: string, needs: string[], artifact: string): DefinedStep {
  return { key, needs, when: null, kind: "approval", artifact };
}

export function input(key: string, needs: string[], artifact: string, when: Condition | null = null): DefinedStep {
  return { key, needs, when, kind: "input", artifact };
}

export function operation(key: string, needs: string[]): DefinedStep {
  return { key, needs, when: null, kind: "operation", operation: key };
}

export function loop(key: string, needs: string[], body: string[], until: Condition): DefinedStep {
  return { key, needs, when: null, kind: "loop", body, until };
}

export function perItem(key: string, needs: string[], items: string, workflow: string): DefinedStep {
  return { key, needs, when: null, kind: "per_item", items, workflow };
}

export function event(kind: string, step: string | null, fields: Partial<TimelineEvent> = {}): TimelineEvent {
  return { at: 0, kind, item: null, step, iteration: null, attempt: null, execution: null, data: {}, ...fields };
}

export function item(key: string, state: ItemStateName, fields: Partial<ItemView> = {}): ItemView {
  return { key, state, step: null, iteration: null, attempt: null, since: null, mergedAs: null, needs: [], waitsFor: [], ...fields };
}

export function gate(step: string, kind: GateSummary["kind"], artifact: string, fields: Partial<GateSummary> = {}): GateSummary {
  return { step, kind, artifact, status: "waiting", revision: 1, decidedBy: null, decidedAt: null, ...fields };
}

export function subject(fields: Partial<SubjectView> = {}): SubjectView {
  return {
    name: "20261107-login",
    run: 1,
    runs: 1,
    states: ["running"],
    endedAt: null,
    driven: true,
    branch: null,
    itemStep: null,
    items: [],
    gates: [],
    blocked: [],
    ...fields,
  };
}

export function detail(steps: DefinedStep[], events: TimelineEvent[] = []): RunDetail {
  return { definition: { name: "feature", source: "schema = 1\n", steps }, events };
}
