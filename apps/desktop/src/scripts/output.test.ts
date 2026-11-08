import { describe, expect, it } from "vitest";
import { MAX_LINES, appendOutput, clean } from "./output";

describe("clean", () => {
  it("drops colour codes and keeps only the last redraw of a progress line", () => {
    expect(clean("\x1b[1m\x1b[92m   Compiling\x1b[0m surt v0.1.0")).toBe("   Compiling surt v0.1.0");
    expect(clean("Building [==>  ] 1/3\rBuilding [====>] 3/3   ")).toBe("Building [====>] 3/3");
  });
});

describe("appendOutput", () => {
  it("skips blank lines and keeps the most recent lines only", () => {
    const blank = appendOutput([], { line: "   ", stream: "stdout" });
    expect(blank).toEqual([]);

    let lines = appendOutput([], { line: "first", stream: "stdout" });
    for (let index = 0; index < MAX_LINES; index += 1) lines = appendOutput(lines, { line: `line ${index}`, stream: "stderr" });

    expect(lines).toHaveLength(MAX_LINES);
    expect(lines[0].text).toBe("line 0");
    expect(lines.at(-1)).toMatchObject({ text: `line ${MAX_LINES - 1}`, stream: "stderr" });
  });
});
