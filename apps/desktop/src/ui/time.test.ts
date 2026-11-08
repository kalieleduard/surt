import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ago, duration } from "./time";

const NOW = 1_800_000_000;

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW * 1000);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("duration", () => {
  it("reads seconds, minutes and hours", () => {
    expect(duration(100, 142)).toBe("42s");
    expect(duration(100, 100 + 5 * 60 + 7)).toBe("5m 7s");
    expect(duration(100, 100 + 2 * 3600 + 15 * 60)).toBe("2h 15m");
  });

  it("measures a running execution up to now and never goes negative", () => {
    expect(duration(NOW - 30, null)).toBe("30s");
    expect(duration(200, 100)).toBe("0s");
  });
});

describe("ago", () => {
  it("rounds to at least one minute, then hours, then days", () => {
    expect(ago(NOW - 10)).toBe("1m ago");
    expect(ago(NOW - 45 * 60)).toBe("45m ago");
    expect(ago(NOW - 5 * 3600)).toBe("5h ago");
    expect(ago(NOW - 3 * 86400)).toBe("3d ago");
  });
});
