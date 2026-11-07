export type SessionEntry =
  | { kind: "context"; text: string }
  | { kind: "agent"; text: string }
  | { kind: "tool"; tool: string; target: string; result?: string };

export const SESSION: SessionEntry[] = [
  { kind: "context", text: "Inputs: spec r2 · task B.md r1 · feedback r1 · worktree B" },
  { kind: "agent", text: "Reading feedback r1. Two findings: the state parameter is not compared in constant time, and the callback ignores provider errors." },
  { kind: "tool", tool: "Read", target: "src/auth/callback.rs" },
  { kind: "tool", tool: "Edit", target: "src/auth/callback.rs", result: "+18 −6" },
  { kind: "agent", text: "Comparing state with constant_time_eq and mapping provider errors to LoginError::Provider." },
  { kind: "tool", tool: "Bash", target: "cargo test -p auth callback", result: "running 18 tests" },
];

export type Question = {
  id: string;
  text: string;
  choices: { label: string; detail: string; recommended?: boolean }[];
};

export const QUESTIONS: { subject: string; step: string; round: number; items: Question[] } = {
  subject: "20261107-2 · billing-export",
  step: "answer-spec",
  round: 1,
  items: [
    {
      id: "q1",
      text: "Which export formats ship first?",
      choices: [
        { label: "CSV only", detail: "Smallest scope, one writer." },
        { label: "CSV and XLSX", detail: "Covers finance teams that live in spreadsheets.", recommended: true },
        { label: "CSV, XLSX and JSON", detail: "Adds an API-friendly format." },
      ],
    },
    {
      id: "q2",
      text: "Who can start an export?",
      choices: [
        { label: "Owners only", detail: "Tightest control." },
        { label: "Owners and admins", detail: "Matches the current billing page.", recommended: true },
        { label: "Any member", detail: "Needs row-level filtering." },
      ],
    },
    {
      id: "q3",
      text: "Where do large exports go?",
      choices: [
        { label: "Email a download link", detail: "Works for exports that take minutes.", recommended: true },
        { label: "In-app download center", detail: "Needs a new page." },
      ],
    },
  ],
};

export const APPROVAL = {
  subject: "20261106-5 · audit-log",
  step: "approve-tasks",
  artifact: "manifest",
  revision: "r1",
  tasks: [
    { key: "A", title: "Audit event schema and migration", deps: [] as string[] },
    { key: "B", title: "Record events from the API layer", deps: ["A"] },
    { key: "C", title: "Retention job", deps: ["A"] },
    { key: "D", title: "Audit log page with filters", deps: ["B"] },
    { key: "E", title: "CSV export of the log", deps: ["D"] },
  ],
};
