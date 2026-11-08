import { describe, expect, it } from "vitest";
import { PARENT_RUN, RUN_INPUT, contractOf, type Specs } from "./contracts";
import { replaceStepBlock, stepBlock } from "./stepBlock";
import { parseToml } from "./toml";

const WORKFLOW = `schema = 1
name = "feature" # trailing comment

[inputs.request]
format = "text"

[steps.spec]
kind = "agent"
agent = "spec"

[steps.approve-spec]
kind = "approval"
artifact = "spec"
rejection_artifact = "spec-rejection-reason"
needs = ["spec"]

[steps.spec-loop]
kind = "loop"
body = ["spec", "approve-spec"]
until = { step = "approve-spec", outcome = "approved" }

[steps.implement]
kind = "per_item"
items = "manifest"
workflow = "task-cycle"
pass = ["spec"]
needs = ["spec-loop"]
`;

const SPEC_AGENT = `
id = "spec"
outcomes = ["done", "questions"]

[inputs.request]
format = "text"

[inputs.spec]
format = "markdown"
optional = true

[inputs.spec-rejection-reason]
format = "text"
optional = true

[outputs.spec]
format = "markdown"

[profile]
model = "claude-opus-5-5"
`;

const specs = (child = false): Specs => ({ workflow: parseToml(WORKFLOW), agents: { spec: parseToml(SPEC_AGENT) }, child });

describe("parseToml", () => {
  it("reads tables, inline tables, arrays, strings and comments", () => {
    const parsed = parseToml(WORKFLOW);

    expect(parsed.name).toBe("feature");
    expect(parsed.steps).toMatchObject({
      "approve-spec": { needs: ["spec"] },
      "spec-loop": { until: { step: "approve-spec", outcome: "approved" } },
    });
    expect(parseToml('[outputs.manifest.item_files.task]\nformat = "markdown"').outputs).toMatchObject({ manifest: { item_files: { task: { format: "markdown" } } } });
    expect(parseToml(`text = """\nline one\nline "two"\n"""\nlist = [\n  1,\n  2, # note\n]\nflag = true`)).toEqual({
      text: 'line one\nline "two"\n',
      list: [1, 2],
      flag: true,
    });
  });

  it("refuses an unterminated string", () => {
    expect(() => parseToml('name = "open')).toThrow();
  });
});

describe("contractOf", () => {
  it("tells where an agent's inputs come from and who reads its outputs", () => {
    const contract = contractOf(specs(), "spec");

    expect(contract.inputs).toEqual([
      { name: "request", format: "text", optional: false, note: undefined, peers: [RUN_INPUT] },
      { name: "spec", format: "markdown", optional: true, note: "its previous revision", peers: [] },
      { name: "spec-rejection-reason", format: "text", optional: true, note: undefined, peers: ["approve-spec"] },
    ]);
    expect(contract.outputs.find((output) => output.name === "spec")?.peers).toEqual(["approve-spec", "implement"]);
    expect(contract.outcomes).toEqual(["done", "questions"]);
  });

  it("describes gates, loops and per item steps", () => {
    expect(contractOf(specs(), "approve-spec")).toMatchObject({
      inputs: [{ name: "spec", format: "markdown", peers: ["spec"] }],
      outputs: [{ name: "spec-rejection-reason", note: "when rejected", peers: ["spec"] }],
      outcomes: ["approved", "rejected"],
    });
    expect(contractOf(specs(), "spec-loop").summary).toContain("spec → approve-spec");
    expect(contractOf(specs(true), "implement").inputs.map((input) => [input.name, input.peers])).toEqual([
      ["manifest", [PARENT_RUN]],
      ["spec", ["spec"]],
    ]);
  });
});

describe("stepBlock", () => {
  it("finds one step's table and replaces only that block", () => {
    expect(stepBlock(WORKFLOW, "approve-spec")?.block).toBe(
      '[steps.approve-spec]\nkind = "approval"\nartifact = "spec"\nrejection_artifact = "spec-rejection-reason"\nneeds = ["spec"]',
    );

    const next = replaceStepBlock(WORKFLOW, "spec", '[steps.spec]\nkind = "agent"\nagent = "writer"\n');

    expect(stepBlock(next, "spec")?.block).toBe('[steps.spec]\nkind = "agent"\nagent = "writer"');
    expect(stepBlock(next, "approve-spec")?.block).toBe(stepBlock(WORKFLOW, "approve-spec")?.block);
    expect(next.split("\n").length).toBe(WORKFLOW.split("\n").length);
  });

  it("appends a step the file does not have yet", () => {
    expect(replaceStepBlock("schema = 1\n", "ship", '[steps.ship]\nkind = "agent"')).toBe('schema = 1\n\n[steps.ship]\nkind = "agent"\n');
  });
});
