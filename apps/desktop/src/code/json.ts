import { scan, type Rule, type Token } from "./tokens";

const RULES: Rule[] = [
  { pattern: /"(?:\\.|[^"\\\n])*"(?=\s*:)/, kind: "key" },
  { pattern: /"(?:\\.|[^"\\\n])*"?/, kind: "string" },
  { pattern: /\b(?:true|false|null)\b/, kind: "keyword" },
  { pattern: /-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/, kind: "number" },
  { pattern: /\/\/[^\n]*/, kind: "comment" },
  { pattern: /[{}[\]:,]/, kind: "punct" },
];

export function jsonTokens(text: string): Token[] {
  return scan(text, RULES);
}
