import type { ScriptOutput } from "../engine/types";

export type OutputLine = { id: number; text: string; stream: ScriptOutput["stream"] };

export const MAX_LINES = 4000;

const ANSI = /\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07]*\x07|\x1b[=>]/g;

let nextId = 0;

export function clean(line: string): string {
  const visible = line.replace(ANSI, "");
  const redrawn = visible.split("\r").filter((segment) => segment.trim()).at(-1) ?? "";
  return redrawn.replace(/\s+$/, "");
}

export function appendOutput(lines: OutputLine[], output: Pick<ScriptOutput, "line" | "stream">): OutputLine[] {
  const text = clean(output.line);
  if (!text && !output.line.trim()) return lines;
  const next = [...lines, { id: nextId++, text, stream: output.stream }];
  return next.length > MAX_LINES ? next.slice(next.length - MAX_LINES) : next;
}
