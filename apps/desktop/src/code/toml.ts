import { scan, split, type Rule, type Token } from "./tokens";

const RULES: Rule[] = [
  { pattern: /#[^\n]*/, kind: "comment" },
  { pattern: /^([ \t]*)(\[\[?[^\]\n]*\]\]?)/m, kind: (match) => split(match, [undefined, "table"]) },
  { pattern: /"""[\s\S]*?(?:"""|$)|'''[\s\S]*?(?:'''|$)/, kind: "string" },
  { pattern: /"(?:\\.|[^"\\\n])*"?|'[^'\n]*'?/, kind: "string" },
  { pattern: /[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]+)*(?=[ \t]*=)/, kind: "key" },
  { pattern: /\b(?:true|false|inf|nan)\b/, kind: "keyword" },
  { pattern: /\d{4}-\d{2}-\d{2}(?:[T ][\d:.]+(?:Z|[+-]\d{2}:\d{2})?)?/, kind: "number" },
  { pattern: /[+-]?(?:0x[\da-fA-F_]+|\d[\d_]*(?:\.\d+)?(?:[eE][+-]?\d+)?)\b/, kind: "number" },
  { pattern: /[=[\]{},.]/, kind: "punct" },
];

export function tomlTokens(text: string): Token[] {
  return scan(text, RULES);
}
