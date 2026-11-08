import { describe, expect, it } from "vitest";
import { definitionChanges, hasChanges } from "./definitionChanges";
import { agent, approval, loop } from "./fixtures";

describe("definitionChanges", () => {
  it("names the steps a newer definition adds, drops and reshapes", () => {
    const frozen = [agent("spec"), approval("approve-spec", ["spec"], "spec"), agent("old")];
    const current = [agent("discovery"), loop("discovery-loop", [], ["discovery"], { step: "discovery", outcome: "done" }), agent("spec", ["discovery-loop"]), approval("approve-spec", ["spec"], "spec")];

    const changes = definitionChanges(frozen, current);

    expect(changes).toEqual({ added: ["discovery", "discovery-loop"], removed: ["old"], changed: ["spec"] });
    expect(hasChanges(changes)).toBe(true);
    expect(hasChanges(definitionChanges(current, current))).toBe(false);
  });
});
