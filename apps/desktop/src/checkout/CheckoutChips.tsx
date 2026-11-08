import { useCallback, useRef, useState } from "react";
import { ChevronDown, FolderGit2, GitBranch } from "lucide-react";
import type { Checkouts, WorktreeView } from "../engine/types";
import { Picker, type PickerSection } from "../ui/Picker";
import { useDismiss } from "../ui/useDismiss";
import { useCheckout } from "./CheckoutContext";
import "./checkout.css";

type Open = "worktree" | "branch" | null;

const BRANCH_NAME = /^(?!-)(?!.*\.\.)(?!.*\/\/)[\w./-]+(?<![./])$/;

export function CheckoutChips() {
  const { checkouts, selected, reload } = useCheckout();
  const [open, setOpen] = useState<Open>(null);
  const ref = useRef<HTMLSpanElement>(null);
  const close = useCallback(() => setOpen(null), []);
  useDismiss(ref, open !== null, close);
  if (!checkouts || !selected) return null;
  const toggle = (which: Open) => {
    if (open !== which) reload();
    setOpen(open === which ? null : which);
  };
  return (
    <span className="checkout" ref={ref}>
      <button className={`checkout__chip${open === "worktree" ? " is-open" : ""}`} onClick={() => toggle("worktree")} title={`Worktree · ${selected.path}`}>
        <FolderGit2 size={13} />
        <span>{selected.main ? "main worktree" : selected.name}</span>
        <ChevronDown size={12} className="checkout__caret" />
      </button>
      <button className={`checkout__chip${open === "branch" ? " is-open" : ""}`} onClick={() => toggle("branch")} title="Branch new runs start from">
        <GitBranch size={13} />
        <span className="mono">{selected.branch ?? `detached ${selected.head}`}</span>
        <ChevronDown size={12} className="checkout__caret" />
      </button>
      {open === "worktree" ? <WorktreePicker checkouts={checkouts} selected={selected} onDone={close} /> : null}
      {open === "branch" ? <BranchPicker checkouts={checkouts} selected={selected} onDone={close} /> : null}
    </span>
  );
}

function useRunning() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = async (work: () => Promise<void>, onDone: () => void) => {
    setBusy(true);
    setError(null);
    try {
      await work();
      onDone();
    } catch (failure) {
      setError(String(failure));
    } finally {
      setBusy(false);
    }
  };
  return { busy, error, run };
}

function WorktreePicker({ checkouts, selected, onDone }: { checkouts: Checkouts; selected: WorktreeView; onDone: () => void }) {
  const { select, createWorktree } = useCheckout();
  const { busy, error, run } = useRunning();
  const base = selected.branch ?? selected.head;
  const sections: PickerSection[] = [
    {
      title: "Worktrees",
      items: checkouts.worktrees.map((worktree) => ({
        key: worktree.path,
        icon: <FolderGit2 size={14} />,
        title: worktree.main ? "main worktree" : worktree.name,
        details: [worktree.branch ?? "detached", worktree.head, shortPath(worktree.path)],
        checked: worktree.path === selected.path,
        onPick: () => {
          select(worktree.path);
          onDone();
        },
      })),
    },
  ];
  return (
    <div className="checkout__popover checkout__popover--worktree">
      <Picker
        placeholder="Select or type a branch to create a worktree…"
        sections={sections}
        busy={busy}
        error={error}
        create={{
          label: (query) => (query && BRANCH_NAME.test(query) && !checkouts.worktrees.some((worktree) => worktree.branch === query) ? `Create worktree for ${query} from ${base}` : null),
          onCreate: (query) => run(() => createWorktree(query), onDone),
        }}
        footer={<span>New runs start from the selected worktree's branch.</span>}
      />
    </div>
  );
}

function BranchPicker({ checkouts, selected, onDone }: { checkouts: Checkouts; selected: WorktreeView; onDone: () => void }) {
  const { select, switchBranch } = useCheckout();
  const { busy, error, run } = useRunning();
  const nameOf = (path: string) => {
    const worktree = checkouts.worktrees.find((candidate) => candidate.path === path);
    return worktree?.main ? "main worktree" : (worktree?.name ?? path);
  };
  const here = checkouts.branches.filter((branch) => branch.worktree === selected.path);
  const elsewhere = checkouts.branches.filter((branch) => branch.worktree && branch.worktree !== selected.path);
  const free = checkouts.branches.filter((branch) => !branch.worktree);
  const sections: PickerSection[] = [
    {
      title: "Current",
      items: here.map((branch) => ({ key: branch.name, icon: <GitBranch size={14} />, title: branch.name, details: [selected.head], checked: true, onPick: onDone })),
    },
    {
      title: `Switch ${selected.main ? "main worktree" : selected.name} to`,
      items: free.map((branch) => ({
        key: branch.name,
        icon: <GitBranch size={14} />,
        title: branch.name,
        details: [],
        onPick: () => run(() => switchBranch(branch.name, false), onDone),
      })),
    },
    {
      title: "Open in its worktree",
      items: elsewhere.map((branch) => ({
        key: branch.name,
        icon: <FolderGit2 size={14} />,
        title: branch.name,
        details: [nameOf(branch.worktree ?? "")],
        onPick: () => {
          if (branch.worktree) select(branch.worktree);
          onDone();
        },
      })),
    },
  ];
  return (
    <div className="checkout__popover checkout__popover--branch">
      <Picker
        placeholder="Switch branch or type to create one…"
        sections={sections}
        busy={busy}
        error={error}
        create={{
          label: (query) => (query && BRANCH_NAME.test(query) && !checkouts.branches.some((branch) => branch.name === query) ? `Create branch ${query} from ${selected.branch ?? selected.head}` : null),
          onCreate: (query) => run(() => switchBranch(query, true), onDone),
        }}
        footer={<span>Switches the branch checked out in {selected.main ? "the main worktree" : selected.name}.</span>}
      />
    </div>
  );
}

function shortPath(path: string) {
  const parts = path.split("/");
  return parts.length > 3 ? `…/${parts.slice(-2).join("/")}` : path;
}
