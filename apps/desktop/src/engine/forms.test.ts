import { describe, expect, it } from "vitest";
import { missingInputs, recommendedAnswers, runInputValues, unanswered } from "./forms";
import type { RunInput, WaitingInput } from "./types";

const WAITING: WaitingInput = {
  step: "answer-spec",
  artifact: "spec-questions",
  revision: 2,
  round: 1,
  questions: [
    { id: "auth", question: "Which login?", why: "Schema.", recommended: "sso", options: [{ value: "sso", description: null }] },
    { id: "scope", question: "Anything else?", why: "Size.", recommended: "no", options: null },
  ],
};

const INPUTS: RunInput[] = [
  { name: "request", format: "text", optional: false, default: null, description: null },
  { name: "base", format: "git_ref", optional: true, default: "HEAD", description: null },
];

describe("answers", () => {
  it("start from each question's recommendation", () => {
    expect(recommendedAnswers(WAITING)).toEqual({ auth: "sso", scope: "no" });
  });

  it("are incomplete while any question is blank", () => {
    expect(unanswered(WAITING, { auth: "password", scope: "  " })).toEqual(["scope"]);
    expect(unanswered(WAITING, recommendedAnswers(WAITING))).toEqual([]);
  });
});

describe("run inputs", () => {
  it("send only the inputs given so optional ones fall back to their default", () => {
    expect(runInputValues(INPUTS, { request: "add login", base: " " })).toEqual([["request", "add login"]]);
    expect(runInputValues(INPUTS, { request: "add login", base: "main" })).toEqual([
      ["request", "add login"],
      ["base", "main"],
    ]);
  });

  it("name every required input left blank", () => {
    expect(missingInputs(INPUTS, { base: "main" })).toEqual(["request"]);
    expect(missingInputs(INPUTS, { request: "add login" })).toEqual([]);
  });
});
