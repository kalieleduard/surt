const HEADER = /^\s*\[\[?([^\]]+)\]\]?\s*(?:#.*)?$/;

type Found = { start: number; end: number; block: string };

export function stepBlock(source: string, key: string): Found | null {
  const lines = source.split("\n");
  const start = lines.findIndex((line) => samePath(headerPath(line), ["steps", key]));
  if (start < 0) return null;
  let end = start + 1;
  while (end < lines.length) {
    const path = headerPath(lines[end]);
    if (path && !(path.length > 2 && samePath(path.slice(0, 2), ["steps", key]))) break;
    end += 1;
  }
  while (end > start + 1 && !lines[end - 1].trim()) end -= 1;
  return { start, end, block: lines.slice(start, end).join("\n") };
}

export function replaceStepBlock(source: string, key: string, block: string): string {
  const found = stepBlock(source, key);
  const clean = block.replace(/\s+$/, "");
  if (!found) return `${source.replace(/\s+$/, "")}\n\n${clean}\n`;
  const lines = source.split("\n");
  return [...lines.slice(0, found.start), clean, ...lines.slice(found.end)].join("\n");
}

function headerPath(line: string): string[] | null {
  const match = HEADER.exec(line);
  if (!match) return null;
  return (match[1].match(/"[^"]*"|'[^']*'|[^.]+/g) ?? []).map((part) => part.trim().replace(/^["']|["']$/g, ""));
}

function samePath(path: string[] | null, expected: string[]) {
  if (!path || path.length !== expected.length) return false;
  return path.every((part, index) => part === expected[index]);
}
