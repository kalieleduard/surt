import { describe, expect, it } from "vitest";
import { subject } from "./fixtures";
import { runControls } from "./runControls";
import type { DriverView } from "./types";

function driver(fields: Partial<DriverView> = {}): DriverView {
  return { subject: "20261107-login", driving: false, ending: null, ...fields };
}

describe("runControls", () => {
  it("offers to continue an interrupted or blocked run that nobody drives", () => {
    for (const state of ["interrupted", "blocked"] as const) {
      expect(runControls(subject({ states: [state], driven: false }), undefined)).toEqual({
        driving: false,
        canContinue: true,
        canStop: false,
        canCancel: true,
        notice: null,
      });
    }
  });

  it("offers to stop a driven run instead of continuing it", () => {
    expect(runControls(subject({ states: ["running"], driven: true }), undefined)).toMatchObject({
      driving: true,
      canContinue: false,
      canStop: true,
    });
  });

  it("does not continue a run waiting on a person, nor one this window is still starting", () => {
    expect(runControls(subject({ states: ["awaiting_approval"], driven: false }), undefined).canContinue).toBe(false);
    expect(runControls(subject({ states: ["interrupted"], driven: false }), driver({ driving: true }))).toMatchObject({
      driving: true,
      canContinue: false,
      canStop: false,
    });
  });

  it("offers nothing for a run that ended", () => {
    for (const state of ["succeeded", "cancelled"] as const) {
      expect(runControls(subject({ states: [state], driven: false }), undefined)).toMatchObject({
        canContinue: false,
        canStop: false,
        canCancel: false,
      });
    }
    expect(runControls(undefined, undefined).canCancel).toBe(false);
  });

  it("surfaces why the last drive from this window could not go on", () => {
    const refused = driver({ ending: { outcome: "refused", message: "git 2.30 is older than 2.38" } });
    const failed = driver({ ending: { outcome: "failed", message: null } });
    const atGate = driver({ ending: { outcome: "at_gate", message: "spec awaits approval" } });

    expect(runControls(subject(), refused).notice).toBe("git 2.30 is older than 2.38");
    expect(runControls(subject(), failed).notice).toBe("the Driver failed");
    expect(runControls(subject(), atGate).notice).toBeNull();
  });
});
