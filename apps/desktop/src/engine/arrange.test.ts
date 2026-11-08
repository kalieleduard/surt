import { describe, expect, it } from "vitest";
import { FRAME_HEADER, FRAME_INSET, LOOP_LANE, MIN_FRAME_CONTENT, NODE_BOX, arrange, framed, layers, stacked } from "./arrange";

const needs: Record<string, string[]> = { plan: ["spec"], build: ["plan"], docs: ["spec"], ship: ["build", "docs"] };
const needsOf = (key: string) => needs[key] ?? [];

describe("layers", () => {
  it("places each step one layer after the deepest step it needs", () => {
    expect(layers(["spec", "plan", "docs", "build", "ship"], needsOf)).toEqual([["spec"], ["plan", "docs"], ["build"], ["ship"]]);
  });

  it("ignores needs outside the group and survives a cycle", () => {
    const cyclic = (key: string) => ({ a: ["b", "outside"], b: ["a"] })[key] ?? [];
    expect(layers(["a", "b"], cyclic).flat().sort()).toEqual(["a", "b"]);
  });
});

describe("stacked", () => {
  it("splits a layer of more than three steps into columns of at most three", () => {
    expect(stacked([["a", "b", "c", "d", "e"], ["f"]])).toEqual([["a", "b", "c"], ["d", "e"], ["f"]]);
  });
});

describe("framed", () => {
  it("adds a return lane below a loop body", () => {
    expect(framed(NODE_BOX, LOOP_LANE).height).toBe(NODE_BOX.height + FRAME_HEADER + FRAME_INSET + LOOP_LANE);
  });

  it("adds the frame inset and header and never narrows below the header width", () => {
    expect(framed(NODE_BOX)).toEqual({
      width: MIN_FRAME_CONTENT + 2 * FRAME_INSET,
      height: NODE_BOX.height + FRAME_HEADER + FRAME_INSET,
    });
    expect(framed({ width: 900, height: 10 }).width).toBe(900 + 2 * FRAME_INSET);
  });
});

describe("arrange", () => {
  it("stacks a column and centres a shorter column beside it", () => {
    const arranged = arrange([["a", "b"], ["c"]], () => NODE_BOX);

    expect(arranged.positions.get("a")).toEqual({ x: 0, y: 0 });
    expect(arranged.positions.get("b")).toEqual({ x: 0, y: 72 });
    expect(arranged.positions.get("c")).toEqual({ x: 280, y: 36 });
    expect(arranged.box).toEqual({ width: 464, height: 120 });
  });

  it("wraps columns that would pass the row width onto a new row", () => {
    const arranged = arrange([["a"], ["b"], ["c"]], () => NODE_BOX, 480);

    expect(arranged.positions.get("b")).toEqual({ x: 280, y: 0 });
    expect(arranged.positions.get("c")).toEqual({ x: 0, y: 120 });
    expect(arranged.box).toEqual({ width: 464, height: 168 });
  });
});
