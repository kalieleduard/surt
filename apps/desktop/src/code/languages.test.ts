import { describe, expect, it } from "vitest";
import { languageOf, tokenize } from "./languages";
import type { Token } from "./tokens";

const kinds = (tokens: Token[]) => tokens.filter((token) => token.kind).map((token) => [token.kind, token.text]);
const joined = (tokens: Token[]) => tokens.map((token) => token.text).join("");

describe("tokenize", () => {
  it("never changes the text it colours", () => {
    const samples: [string, Parameters<typeof tokenize>[1]][] = [
      ['[steps.spec]\nkind = "agent" # why\nneeds = ["a", \'b\']\nn = 1_000\n', "toml"],
      ['<task id="1">\n  <!-- note -->\n  a &amp; b <b/>\n</task>', "xml"],
      ["---\ntitle: x\n---\n# Head\n- item `code` **bold**\n```toml\nkey = 1\n```\n> quote", "markdown"],
      ['{ "a": [1, true, null, "s"] }', "json"],
    ];
    for (const [text, language] of samples) expect(joined(tokenize(text, language))).toBe(text);
  });

  it("colours TOML tables, keys, strings, numbers and comments", () => {
    expect(kinds(tokenize('[steps.spec]\nkind = "agent" # why\nlimit = 3\nok = true', "toml"))).toEqual([
      ["table", "[steps.spec]"],
      ["key", "kind"],
      ["punct", "="],
      ["string", '"agent"'],
      ["comment", "# why"],
      ["key", "limit"],
      ["punct", "="],
      ["number", "3"],
      ["key", "ok"],
      ["punct", "="],
      ["keyword", "true"],
    ]);
  });

  it("colours XML tags, attributes and comments", () => {
    expect(kinds(tokenize('<task id="1"><!-- c --></task>', "xml"))).toEqual([
      ["punct", "<"],
      ["tag", "task"],
      ["attr", "id"],
      ["punct", "="],
      ["string", '"1"'],
      ["punct", ">"],
      ["comment", "<!-- c -->"],
      ["punct", "</"],
      ["tag", "task"],
      ["punct", ">"],
    ]);
  });

  it("colours Markdown structure and the languages inside fences", () => {
    const tokens = kinds(tokenize("# Title\nUse `surt run`.\n```toml\nkey = 1\n```\n<rules>", "markdown"));

    expect(tokens).toContainEqual(["heading", "# Title"]);
    expect(tokens).toContainEqual(["code", "`surt run`"]);
    expect(tokens).toContainEqual(["key", "key"]);
    expect(tokens).toContainEqual(["tag", "rules"]);
  });

  it("picks the language from the file extension", () => {
    expect(["a.toml", "instructions.md", "x.xml", "s.schema.json", "notes.txt"].map(languageOf)).toEqual(["toml", "markdown", "xml", "json", "text"]);
  });
});
