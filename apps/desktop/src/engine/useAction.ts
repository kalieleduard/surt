import { useCallback, useState } from "react";
import { useEngine } from "./EngineContext";

export type Done<T> = { ok: true; value: T } | { ok: false };

export function useAction() {
  const { refresh } = useEngine();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = useCallback(
    async <T,>(work: () => Promise<T>): Promise<Done<T>> => {
      setBusy(true);
      setError(null);
      try {
        const value = await work();
        refresh();
        return { ok: true, value };
      } catch (failure) {
        setError(String(failure));
        return { ok: false };
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );
  return { busy, error, run };
}
