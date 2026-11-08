import type { RunInput, WaitingInput } from "./types";

export function recommendedAnswers(waiting: WaitingInput): Record<string, string> {
  return Object.fromEntries(waiting.questions.map((question) => [question.id, question.recommended]));
}

export function unanswered(waiting: WaitingInput, answers: Record<string, string>): string[] {
  return waiting.questions.filter((question) => !answers[question.id]?.trim()).map((question) => question.id);
}

export function runInputValues(inputs: RunInput[], values: Record<string, string>): [string, string][] {
  return inputs.filter((input) => values[input.name]?.trim()).map((input) => [input.name, values[input.name]]);
}

export function missingInputs(inputs: RunInput[], values: Record<string, string>): string[] {
  return inputs.filter((input) => !input.optional && !values[input.name]?.trim()).map((input) => input.name);
}
