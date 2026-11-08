import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { engine } from "../engine/client";
import { useEngine } from "../engine/EngineContext";
import type { Checkouts, WorktreeView } from "../engine/types";

type Checkout = {
  checkouts: Checkouts | null;
  error: string | null;
  selected: WorktreeView | null;
  baseRef: string | null;
  reload: () => void;
  select: (path: string) => void;
  switchBranch: (branch: string, create: boolean) => Promise<void>;
  createWorktree: (branch: string) => Promise<void>;
};

const CheckoutContext = createContext<Checkout | null>(null);
const SELECTED_PREFIX = "surt-checkout";

export function useCheckout() {
  const value = useContext(CheckoutContext);
  if (!value) throw new Error("useCheckout needs a CheckoutProvider");
  return value;
}

export function CheckoutProvider({ children }: { children: ReactNode }) {
  const { project, available } = useEngine();
  const [checkouts, setCheckouts] = useState<Checkouts | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  const storageKey = project ? `${SELECTED_PREFIX}:${project.root}` : null;

  const reload = useCallback(() => {
    if (!available || !project) return;
    engine
      .checkouts()
      .then((next) => {
        setCheckouts(next);
        setError(null);
      })
      .catch((failure) => setError(String(failure)));
  }, [available, project]);

  useEffect(() => {
    setCheckouts(null);
    setChosen(storageKey ? recall(storageKey) : null);
    reload();
  }, [reload, storageKey]);

  const selected = checkouts ? (checkouts.worktrees.find((worktree) => worktree.path === chosen) ?? checkouts.worktrees.find((worktree) => worktree.main) ?? null) : null;

  const select = useCallback(
    (path: string) => {
      setChosen(path);
      if (storageKey) remember(storageKey, path);
    },
    [storageKey],
  );

  const apply = useCallback(async (change: () => Promise<Checkouts>) => {
    const next = await change();
    setCheckouts(next);
    setError(null);
    return next;
  }, []);

  const value = useMemo<Checkout>(
    () => ({
      checkouts,
      error,
      selected,
      baseRef: selected ? (selected.branch ?? selected.head) : null,
      reload,
      select,
      switchBranch: async (branch, create) => {
        if (!selected) return;
        await apply(() => engine.switchBranch(selected.path, branch, create));
      },
      createWorktree: async (branch) => {
        const next = await apply(() => engine.createWorktree(branch, selected ? (selected.branch ?? selected.head) : null));
        const created = next.worktrees.find((worktree) => worktree.branch === branch.trim());
        if (created) select(created.path);
      },
    }),
    [checkouts, error, selected, reload, select, apply],
  );

  return <CheckoutContext.Provider value={value}>{children}</CheckoutContext.Provider>;
}

function recall(key: string) {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function remember(key: string, path: string) {
  try {
    localStorage.setItem(key, path);
  } catch {
    return;
  }
}
