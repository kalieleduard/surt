import { useEffect, useRef, useState } from "react";

export const POLL_INTERVAL_MS = 2000;

export type Polled<T> = { data: T | null; error: string | null; refresh: () => void };

export function usePolled<T>(load: (() => Promise<T>) | null, key: string): Polled<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const last = useRef<string | null>(null);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    setData(null);
    setError(null);
    last.current = null;
  }, [key]);

  useEffect(() => {
    if (!load) return;
    let cancelled = false;
    const read = async () => {
      if (document.hidden) return;
      try {
        const next = await load();
        const text = JSON.stringify(next);
        if (cancelled) return;
        setError(null);
        if (text === last.current) return;
        last.current = text;
        setData(next);
      } catch (failure) {
        if (!cancelled) setError(String(failure));
      }
    };
    read();
    const timer = window.setInterval(read, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [load, key, tick]);

  return { data, error, refresh: () => setTick((value) => value + 1) };
}
