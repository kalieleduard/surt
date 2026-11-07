import type { StepState } from "./types";

export type LogLine = { time: string; state: StepState | "info"; symbol: string; text: string };

export const LOGS: LogLine[] = [
  { time: "09:02:11", state: "running", symbol: "●", text: "spec started · iteration 1 · attempt 1" },
  { time: "09:06:40", state: "waiting", symbol: "?", text: "spec asked 3 questions → answer-spec" },
  { time: "09:13:21", state: "info", symbol: "✎", text: "spec-answers r1 recorded" },
  { time: "09:19:02", state: "succeeded", symbol: "✔", text: "spec delivered spec r2 · outcome done" },
  { time: "09:30:12", state: "succeeded", symbol: "✔", text: "approve-spec approved spec r2" },
  { time: "09:36:00", state: "succeeded", symbol: "✔", text: "plan-tasks delivered manifest r1 · 4 tasks" },
  { time: "09:39:05", state: "succeeded", symbol: "✔", text: "approve-tasks approved manifest r1" },
  { time: "09:39:06", state: "info", symbol: "→", text: "prepare created surt/20261107-3 at b72ec4f" },
  { time: "09:44:30", state: "waiting", symbol: "⏸", text: "rate limit · agent launches paused until 09:47" },
  { time: "09:58:41", state: "succeeded", symbol: "✔", text: "integrate[A] merged into surt/20261107-3" },
  { time: "10:03:52", state: "blocked", symbol: "✖", text: "build[B] invalid output: missing commit" },
  { time: "10:04:12", state: "running", symbol: "↻", text: "build[B] attempt 2 · continues conversation" },
  { time: "10:05:30", state: "running", symbol: "●", text: "review[C] started · reading 3f9a1c2" },
];

export type TimelineBar = {
  lane: string;
  start: number;
  end: number;
  label: string;
  state: StepState;
};

export const TIMELINE_START = "09:00";
export const TIMELINE_MINUTES = 72;
export const TIMELINE_NOW = 68;
export const TIMELINE_LANES = ["Slot 1", "Slot 2", "Slot 3", "Human"];
export const TRANSIENT_PAUSE = { start: 44, end: 47, label: "rate limit" };

export const TIMELINE: TimelineBar[] = [
  { lane: "Slot 1", start: 2, end: 6.5, label: "spec #1", state: "succeeded" },
  { lane: "Slot 1", start: 13.5, end: 19, label: "spec #2", state: "succeeded" },
  { lane: "Slot 1", start: 31, end: 36, label: "plan-tasks", state: "succeeded" },
  { lane: "Slot 1", start: 40, end: 52, label: "build A", state: "succeeded" },
  { lane: "Slot 1", start: 53, end: 57, label: "review A", state: "succeeded" },
  { lane: "Slot 1", start: 64, end: TIMELINE_NOW, label: "build B · attempt 2", state: "running" },
  { lane: "Slot 2", start: 40, end: 50, label: "build B #1", state: "succeeded" },
  { lane: "Slot 2", start: 51, end: 56, label: "review B", state: "succeeded" },
  { lane: "Slot 2", start: 58, end: 63.8, label: "build B · attempt 1", state: "blocked" },
  { lane: "Slot 3", start: 40, end: 55, label: "build C", state: "succeeded" },
  { lane: "Slot 3", start: 65.5, end: TIMELINE_NOW, label: "review C", state: "running" },
  { lane: "Human", start: 6.5, end: 13.3, label: "answer-spec", state: "waiting" },
  { lane: "Human", start: 19, end: 30.2, label: "approve-spec", state: "waiting" },
  { lane: "Human", start: 36, end: 39, label: "approve-tasks", state: "waiting" },
];
