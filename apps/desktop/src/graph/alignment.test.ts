import { describe, expect, it } from "vitest";
import { align, type Rect } from "./alignment";

const rects: Rect[] = [
  { id: "a", x: 0, y: 10, width: 100, height: 40 },
  { id: "b", x: 300, y: 0, width: 100, height: 60 },
  { id: "c", x: 140, y: 100, width: 60, height: 40 },
];

const positions = (alignment: Parameters<typeof align>[1], gap?: number) => Object.fromEntries(align(rects, alignment, gap));

describe("align", () => {
  it("lines up left edges on the leftmost one", () => {
    expect(positions("left")).toEqual({ a: { x: 0, y: 10 }, b: { x: 0, y: 0 }, c: { x: 0, y: 100 } });
  });

  it("lines up right edges on the rightmost one", () => {
    expect(positions("right")).toEqual({ a: { x: 300, y: 10 }, b: { x: 300, y: 0 }, c: { x: 340, y: 100 } });
  });

  it("centers vertically on the middle of the selection", () => {
    expect(positions("middle")).toEqual({ a: { x: 0, y: 50 }, b: { x: 300, y: 40 }, c: { x: 140, y: 50 } });
  });

  it("spreads boxes so every gap between them is equal, keeping the outer ones", () => {
    expect(positions("distribute-x")).toEqual({ a: { x: 0, y: 10 }, c: { x: 170, y: 100 }, b: { x: 300, y: 0 } });
  });

  it("puts boxes side by side with a uniform gap, centered on one line", () => {
    expect(positions("row", 20)).toEqual({ a: { x: 0, y: 50 }, c: { x: 120, y: 50 }, b: { x: 200, y: 40 } });
  });

  it("stacks boxes in a column with a uniform gap, centered on one axis", () => {
    expect(positions("column", 10)).toEqual({ b: { x: 150, y: 0 }, a: { x: 150, y: 70 }, c: { x: 170, y: 120 } });
  });

  it("leaves a single box where it is", () => {
    expect(Object.fromEntries(align(rects.slice(0, 1), "center"))).toEqual({ a: { x: 0, y: 10 } });
  });
});
