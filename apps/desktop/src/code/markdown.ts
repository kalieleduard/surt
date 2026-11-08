import { jsonTokens } from "./json";
import { scan, split, type Rule, type Token } from "./tokens";
import { tomlTokens } from "./toml";
import { xmlTokens } from "./xml";

const FENCE = /^[ \t]*(`{3,}|~{3,})[ \t]*([\w+-]*)/;

const INLINE: Rule[] = [
  { pattern: /`[^`\n]+`/, kind: "code" },
  { pattern: /(\*\*|__)(?=\S)[^\n]*?\S\1/, kind: "strong" },
  { pattern: /(?<![\w*])([*_])(?=\S)[^\n*_]*?\S\1(?![\w*])/, kind: "em" },
  { pattern: /(!?\[)([^\]\n]*)(\]\()([^)\n]*)(\))/, kind: (match) => split(match, ["punct", "link", "punct", "url", "punct"]) },
  { pattern: /<!--[\s\S]*?(?:-->|$)/, kind: "comment" },
  { pattern: /<\/?[A-Za-z][\w:.-]*(?:\s[^<>\n]*)?\/?>/, kind: (match) => xmlTokens(match[0]) },
  { pattern: /https?:\/\/[^\s)>\]]+/, kind: "url" },
];

const BLOCK: [RegExp, (match: RegExpExecArray) => Token[]][] = [
  [/^#{1,6}[ \t].*$/, (match) => [{ text: match[0], kind: "heading" }]],
  [/^([ \t]*>[ \t]?)(.*)$/, (match) => [{ text: match[1], kind: "marker" }, ...quoted(scan(match[2], INLINE))]],
  [/^([ \t]*)([-*_])(?:[ \t]*\2){2,}[ \t]*$/, (match) => [{ text: match[0], kind: "marker" }]],
  [/^([ \t]*)([-*+]|\d+[.)])([ \t]+(?:\[[ xX]\][ \t]+)?)(.*)$/, (match) => [...split(match, [undefined, "marker", "marker"]), ...scan(match[4], INLINE)]],
];

const NESTED: Record<string, (text: string) => Token[]> = { toml: tomlTokens, xml: xmlTokens, html: xmlTokens, json: jsonTokens };

export function markdownTokens(text: string): Token[] {
  const lines = text.split("\n");
  const tokens: Token[] = [];
  let index = frontMatter(lines, tokens);
  while (index < lines.length) {
    const fence = FENCE.exec(lines[index]);
    if (fence) {
      index = codeBlock(lines, index, fence, tokens);
      continue;
    }
    tokens.push(...line(lines[index]));
    if (index < lines.length - 1) tokens.push({ text: "\n" });
    index += 1;
  }
  return tokens;
}

function frontMatter(lines: string[], tokens: Token[]): number {
  if (lines[0] !== "---") return 0;
  const end = lines.indexOf("---", 1);
  if (end < 0) return 0;
  tokens.push({ text: lines.slice(0, end + 1).join("\n"), kind: "meta" });
  if (end < lines.length - 1) tokens.push({ text: "\n" });
  return end + 1;
}

function codeBlock(lines: string[], start: number, fence: RegExpExecArray, tokens: Token[]): number {
  let end = start + 1;
  while (end < lines.length && !lines[end].trimStart().startsWith(fence[1])) end += 1;
  const body = lines.slice(start + 1, end).join("\n");
  const nested = NESTED[fence[2].toLowerCase()];
  tokens.push({ text: lines[start], kind: "marker" });
  if (end > start + 1) tokens.push({ text: "\n" }, ...(nested ? nested(body) : [{ text: body, kind: "code" as const }]));
  if (end < lines.length) tokens.push({ text: "\n" }, { text: lines[end], kind: "marker" });
  if (end < lines.length - 1) tokens.push({ text: "\n" });
  return end + 1;
}

function line(text: string): Token[] {
  for (const [pattern, emit] of BLOCK) {
    const match = pattern.exec(text);
    if (match) return emit(match);
  }
  return scan(text, INLINE);
}

function quoted(tokens: Token[]): Token[] {
  return tokens.map((token) => (token.kind ? token : { ...token, kind: "quote" }));
}
