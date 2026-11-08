import { jsonTokens } from "./json";
import { markdownTokens } from "./markdown";
import type { Token } from "./tokens";
import { tomlTokens } from "./toml";
import { xmlTokens } from "./xml";

export type Language = "toml" | "markdown" | "xml" | "json" | "text";

const BY_EXTENSION: Record<string, Language> = {
  toml: "toml",
  md: "markdown",
  markdown: "markdown",
  xml: "xml",
  html: "xml",
  svg: "xml",
  xsd: "xml",
  json: "json",
};

const TOKENIZERS: Record<Language, (text: string) => Token[]> = {
  toml: tomlTokens,
  markdown: markdownTokens,
  xml: xmlTokens,
  json: jsonTokens,
  text: (text) => [{ text }],
};

export function languageOf(path: string): Language {
  const extension = path.slice(path.lastIndexOf(".") + 1).toLowerCase();
  return BY_EXTENSION[extension] ?? "text";
}

export function tokenize(text: string, language: Language): Token[] {
  return TOKENIZERS[language](text);
}
