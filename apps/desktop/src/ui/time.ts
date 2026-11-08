const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

export function duration(startedAt: number, endedAt: number | null) {
  const seconds = Math.max(0, Math.round((endedAt ?? Date.now() / 1000) - startedAt));
  if (seconds < MINUTE) return `${seconds}s`;
  if (seconds < HOUR) return `${Math.floor(seconds / MINUTE)}m ${seconds % MINUTE}s`;
  return `${Math.floor(seconds / HOUR)}h ${Math.floor((seconds % HOUR) / MINUTE)}m`;
}

export function moment(seconds: number) {
  return new Date(seconds * 1000).toLocaleString(undefined, { dateStyle: "short", timeStyle: "short" });
}

export function ago(seconds: number) {
  const elapsed = Math.max(0, Date.now() / 1000 - seconds);
  if (elapsed < HOUR) return `${Math.max(1, Math.floor(elapsed / MINUTE))}m ago`;
  if (elapsed < DAY) return `${Math.floor(elapsed / HOUR)}h ago`;
  return `${Math.floor(elapsed / DAY)}d ago`;
}
