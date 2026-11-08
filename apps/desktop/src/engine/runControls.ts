import type { DriverView, RunStateName, SubjectView } from "./types";

export type RunControls = {
  driving: boolean;
  canContinue: boolean;
  canStop: boolean;
  canCancel: boolean;
  notice: string | null;
};

const TERMINAL: RunStateName[] = ["succeeded", "cancelled"];
const RESUMABLE: RunStateName[] = ["interrupted", "blocked", "running"];
const FAILED_ENDINGS = ["refused", "failed"];

export function runControls(subject: SubjectView | undefined, driver: DriverView | undefined): RunControls {
  if (!subject) return { driving: false, canContinue: false, canStop: false, canCancel: false, notice: null };
  const active = !subject.states.some((state) => TERMINAL.includes(state));
  const driving = subject.driven || Boolean(driver?.driving);
  return {
    driving,
    canContinue: active && !driving && subject.states.some((state) => RESUMABLE.includes(state)),
    canStop: subject.driven,
    canCancel: active,
    notice: driverNotice(driver),
  };
}

function driverNotice(driver: DriverView | undefined): string | null {
  const ending = driver?.ending;
  if (!driver || driver.driving || !ending || !FAILED_ENDINGS.includes(ending.outcome)) return null;
  return ending.message ?? `the Driver ${ending.outcome}`;
}
