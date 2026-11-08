import { OPERATIONS } from "./operations";
import type { TomlTable, TomlValue } from "./toml";

export type Flow = { name: string; format?: string; optional?: boolean; note?: string; peers: string[] };
export type Contract = { summary: string; inputs: Flow[]; outputs: Flow[]; outcomes: string[] };
export type Specs = { workflow: TomlTable; agents: Record<string, TomlTable>; child: boolean };

type Piece = Omit<Flow, "peers">;

export const RUN_INPUT = "run input";
export const PARENT_RUN = "parent run";

export function table(value: TomlValue | undefined): TomlTable {
  return value && typeof value === "object" && !Array.isArray(value) ? value : {};
}

export function text(value: TomlValue | undefined): string | undefined {
  return typeof value === "string" ? value : undefined;
}

export function list(value: TomlValue | undefined): string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === "string") : [];
}

export function stepsOf(specs: Specs): [string, TomlTable][] {
  return Object.entries(table(specs.workflow.steps)).map(([key, value]) => [key, table(value)]);
}

export function contractOf(specs: Specs, key: string): Contract {
  const step = table(table(specs.workflow.steps)[key]);
  const producers = index(specs, (raw) => produced(raw, specs));
  for (const name of Object.keys(table(specs.workflow.inputs))) producers.set(name, [RUN_INPUT, ...(producers.get(name) ?? [])]);
  const consumers = index(specs, (raw) => consumed(raw, specs));
  const formats = formatsOf(specs);
  const inputs = consumed(step, specs).map((piece) => {
    const peers = (producers.get(piece.name) ?? []).filter((peer) => peer !== key);
    const own = producers.get(piece.name)?.includes(key);
    const fallback = !peers.length && specs.child ? [PARENT_RUN] : [];
    return { ...piece, format: piece.format ?? formats.get(piece.name), note: piece.note ?? (own ? "its previous revision" : undefined), peers: peers.length ? peers : fallback };
  });
  const outputs = produced(step, specs).map((piece) => ({
    ...piece,
    format: piece.format ?? formats.get(piece.name),
    peers: (consumers.get(piece.name) ?? []).filter((peer) => peer !== key),
  }));
  return { summary: summaryOf(step), inputs, outputs, outcomes: outcomesOf(step, specs) };
}

function index(specs: Specs, pieces: (raw: TomlTable) => Piece[]): Map<string, string[]> {
  const found = new Map<string, string[]>();
  for (const [key, raw] of stepsOf(specs)) {
    for (const piece of pieces(raw)) found.set(piece.name, [...(found.get(piece.name) ?? []), key]);
  }
  return found;
}

function produced(step: TomlTable, specs: Specs): Piece[] {
  const kind = text(step.kind);
  if (kind === "agent") {
    return Object.entries(table(agentOf(step, specs).outputs)).map(([name, value]) => {
      const outcomes = list(table(value).outcomes);
      return { name, format: text(table(value).format), note: outcomes.length ? `when ${outcomes.join(" or ")}` : undefined };
    });
  }
  if (kind === "approval") return named(text(step.rejection_artifact), "text", "when rejected");
  if (kind === "input") return named(text(step.answer_artifact), "json", "one round per answer");
  if (kind === "operation") return (OPERATIONS[text(step.run) ?? ""]?.produces ?? []).map((name) => ({ name }));
  return [];
}

function consumed(step: TomlTable, specs: Specs): Piece[] {
  const kind = text(step.kind);
  if (kind === "agent") {
    return Object.entries(table(agentOf(step, specs).inputs)).map(([name, value]) => ({
      name,
      format: text(table(value).format),
      optional: table(value).optional === true,
    }));
  }
  if (kind === "approval") return named(text(step.artifact), undefined, "the revision to decide on");
  if (kind === "input") return named(text(step.artifact), "questions", "the questions to answer");
  if (kind === "operation") return (OPERATIONS[text(step.run) ?? ""]?.inputs ?? []).map((name) => ({ name }));
  if (kind === "per_item") {
    const passed = list(step.pass).map((name) => ({ name, note: "passed to every child run" }));
    return [...named(text(step.items), "item_list", "one child run per item"), ...passed];
  }
  return [];
}

function outcomesOf(step: TomlTable, specs: Specs): string[] {
  const kind = text(step.kind);
  if (kind === "agent") return list(agentOf(step, specs).outcomes);
  if (kind === "approval") return ["approved", "rejected"];
  if (kind === "input") return ["answered"];
  if (kind === "operation") return OPERATIONS[text(step.run) ?? ""]?.outcomes ?? [];
  return [];
}

function summaryOf(step: TomlTable): string {
  const kind = text(step.kind);
  if (kind === "agent") return `Runs the ${text(step.agent)} agent with the inputs below and records what it delivers.`;
  if (kind === "approval") return `Pauses the run until someone approves or rejects ${text(step.artifact)}.`;
  if (kind === "input") return `Pauses the run until someone answers the questions in ${text(step.artifact)}.`;
  if (kind === "operation") return OPERATIONS[text(step.run) ?? ""]?.summary ?? `Runs the ${text(step.run)} operation.`;
  if (kind === "loop") {
    const until = table(step.until);
    return `Repeats ${list(step.body).join(" → ")} as new iterations until ${text(until.step)} ends ${text(until.outcome)}.`;
  }
  if (kind === "per_item") return `Starts one ${text(step.workflow)} run for each item of ${text(step.items)}, in dependency order.`;
  return "";
}

function formatsOf(specs: Specs): Map<string, string> {
  const formats = new Map<string, string>();
  const note = (entries: TomlTable) => {
    for (const [name, value] of Object.entries(entries)) {
      const format = text(table(value).format);
      if (format && !formats.has(name)) formats.set(name, format);
    }
  };
  note(table(specs.workflow.inputs));
  for (const agent of Object.values(specs.agents)) note(table(agent.outputs));
  for (const agent of Object.values(specs.agents)) note(table(agent.inputs));
  return formats;
}

function agentOf(step: TomlTable, specs: Specs): TomlTable {
  return specs.agents[text(step.agent) ?? ""] ?? {};
}

function named(name: string | undefined, format: string | undefined, note: string): Piece[] {
  return name ? [{ name, format, note }] : [];
}
