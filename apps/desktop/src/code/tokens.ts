export type TokenKind =
  | "comment"
  | "string"
  | "number"
  | "keyword"
  | "key"
  | "table"
  | "punct"
  | "tag"
  | "attr"
  | "entity"
  | "heading"
  | "strong"
  | "em"
  | "code"
  | "link"
  | "url"
  | "quote"
  | "marker"
  | "meta";

export type Token = { text: string; kind?: TokenKind };

export type Rule = { pattern: RegExp; kind: TokenKind | ((match: RegExpExecArray) => Token[]) };

const STICKY = new WeakMap<Rule[], Rule[]>();

function stickyRules(rules: Rule[]): Rule[] {
  const known = STICKY.get(rules);
  if (known) return known;
  const compiled = rules.map((rule) => ({ ...rule, pattern: new RegExp(rule.pattern.source, `${rule.pattern.flags.replace("g", "")}y`) }));
  STICKY.set(rules, compiled);
  return compiled;
}

export function scan(text: string, rules: Rule[]): Token[] {
  const sticky = stickyRules(rules);
  const tokens: Token[] = [];
  let plain = "";
  let at = 0;
  while (at < text.length) {
    const hit = firstMatch(sticky, text, at);
    if (!hit) {
      plain += text[at];
      at += 1;
      continue;
    }
    if (plain) tokens.push({ text: plain });
    plain = "";
    const [rule, match] = hit;
    if (typeof rule.kind === "function") tokens.push(...rule.kind(match));
    else tokens.push({ text: match[0], kind: rule.kind });
    at += match[0].length;
  }
  if (plain) tokens.push({ text: plain });
  return tokens;
}

function firstMatch(rules: Rule[], text: string, at: number): [Rule, RegExpExecArray] | null {
  for (const rule of rules) {
    rule.pattern.lastIndex = at;
    const match = rule.pattern.exec(text);
    if (match && match[0].length) return [rule, match];
  }
  return null;
}

export function split(match: RegExpExecArray, kinds: (TokenKind | undefined)[]): Token[] {
  return kinds.flatMap((kind, index) => (match[index + 1] ? [{ text: match[index + 1], kind }] : []));
}
