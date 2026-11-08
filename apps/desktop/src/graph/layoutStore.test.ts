import { describe, expect, it } from "vitest";
import { merged } from "./layoutStore";

describe("merged", () => {
  it("keeps a resized step's size when it is moved later", () => {
    const resized = merged({}, { spec: { x: 10, y: 20, width: 260, height: 80 } });

    expect(merged(resized, { spec: { x: 40, y: 20 }, plan: { x: 0, y: 0 } })).toEqual({
      spec: { x: 40, y: 20, width: 260, height: 80 },
      plan: { x: 0, y: 0 },
    });
  });
});
