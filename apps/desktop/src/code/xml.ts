import { scan, split, type Rule, type Token } from "./tokens";

const INSIDE: Rule[] = [
  { pattern: /[\w:.-]+(?=\s*=)/, kind: "attr" },
  { pattern: /"[^"]*"?|'[^']*'?/, kind: "string" },
  { pattern: /=/, kind: "punct" },
];

const RULES: Rule[] = [
  { pattern: /<!--[\s\S]*?(?:-->|$)/, kind: "comment" },
  { pattern: /<!\[CDATA\[[\s\S]*?(?:\]\]>|$)/, kind: "string" },
  { pattern: /<[?!][^>]*>?/, kind: "meta" },
  { pattern: /(<\/?)([\w:.-]+)([^<>]*?)(\/?>|$)/, kind: (match) => tag(match) },
  { pattern: /&(?:#\d+|#x[\da-fA-F]+|\w+);/, kind: "entity" },
];

function tag(match: RegExpExecArray): Token[] {
  const close: Token[] = match[4] ? [{ text: match[4], kind: "punct" }] : [];
  return [...split(match, ["punct", "tag"]), ...scan(match[3] ?? "", INSIDE), ...close];
}

export function xmlTokens(text: string): Token[] {
  return scan(text, RULES);
}
