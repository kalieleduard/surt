# Manual smoke: the real Claude Code on macOS

This checklist makes "Milestone 1 done" a repeatable check ([K. Roadmap](architecture/K-roadmap.md), criterion 2): in a toy repository, a Feature with two Tasks reaches `succeeded` with the real Claude Code. Every section uses the CLI of the current binary (since Milestone 5: `surt run` and `surt continue`, with the feature workflow written into the toy repository by `surt init --template feature`). Everything else in Milestone 1 is covered by the automated suite with the Fake Runtime and stub executables; this is the only check that runs the real `claude`.

**It spends your Claude subscription.** A full run launches about seven agent Executions (spec, tasks, two Builders, two Reviewers, the Shipper), more if a review asks for changes. Run it deliberately, not in a loop.

Work through the boxes in order. Every command runs in the toy repository unless stated otherwise. `<name>` is the Feature name `surt run` prints (for example `20261018-greet-and-farewell`).

Since Milestone 6 the Spec Agent may ask questions before it writes the spec. Wherever a section below expects the spec gate and a run instead stops at `? spec-questions rev N awaiting answers (round R)`, run `"$SURT" answer <name> --accept`: it takes every recommendation and drives on like `surt continue`, to another round or to the spec gate. Only the Milestone 6 section checks the questions themselves.

## 1. Prerequisites

- [ ] macOS with Git 2.38 or newer: `git --version`.
- [ ] Claude Code **2.1.301 or newer** on `PATH`: `claude --version` prints e.g. `2.1.301 (Claude Code)`. Upgrade with `claude update`.
- [ ] Claude Code is logged in with a **subscription** (run `claude` once interactively and `/login` if needed). Surt never passes API keys, gateways or `ANTHROPIC_*` variables through, so an API-key-only setup fails with `authentication_failed`.
- [ ] The subscription can use the model of the feature Template's Agents, `claude-sonnet-5-5` (see `crates/surt/assets/templates/feature/agents/*/agent.toml`, which `surt init --template feature` writes to `.surt/agents/*/agent.toml`).
- [ ] No other `claude -p` process is running: `pgrep -fl 'claude -p'` prints nothing. Later steps use this to detect leftovers.

## 2. Build the release binary

From the Surt checkout:

- [ ] `cargo build --release -p surt` succeeds. Do **not** pass `--features test-support`: the release binary has no Fake Runtime and always uses Claude Code.
- [ ] `export SURT="$PWD/target/release/surt"` and `"$SURT" --help` lists `init`, `run`, `continue`, `approve`, `reject`.
- [ ] Record the Surt commit: `git rev-parse --short HEAD`.

## 3. Set up the toy repository

- [ ] Create it outside the Surt checkout:

  ```sh
  mkdir -p ~/surt-smoke && cd ~/surt-smoke && rm -rf toy && mkdir toy && cd toy
  git init --quiet -b main
  cat > test.sh <<'SH'
  #!/bin/sh
  set -e
  for t in tests/*.sh; do
    [ -e "$t" ] || continue
    sh "$t"
  done
  echo "all tests passed"
  SH
  mkdir tests
  printf '# Toy\n\nShell scripts that print messages.\n' > README.md
  git add . && git commit --quiet -m "toy: empty project"
  ```

- [ ] `"$SURT" init --template feature` prints a success line, one `created` line per file (`.surt/.gitignore`, `.surt/config.toml`, `.surt/surt.db`, `.surt/workflows/feature.toml`, `.surt/workflows/task-cycle.toml` and the files of the five Agents under `.surt/agents/`), and `next: surt run feature --input request="<what you want to build>"`.
- [ ] Add the Check, so every Builder commit and every integration runs the tests:

  ```sh
  cat >> .surt/config.toml <<'TOML'

  [[checks]]
  name = "test"
  run = ["sh", "test.sh"]
  timeout = "2m"
  TOML
  git add .surt && git commit --quiet -m "surt: config"
  ```

- [ ] `git status --short` is empty: the commit holds `config.toml`, `workflows/` and `agents/`, and `.surt/.gitignore` keeps the runtime state out of Git.

## 4. The spec

- [ ] Create the Feature:

  ```sh
  "$SURT" run feature --input request="Add greet.sh, which prints 'Hello, <name>!' for its first argument, and farewell.sh, which prints 'Goodbye, <name>!'. Each script gets its own test under tests/."
  ```

  Expected: `✔ created <name>   workflow: feature   base: main @ <sha7>`, then `● spec started`, then `⏸ spec rev 1 awaiting approval in .surt/subjects/<name>` with the `approve:` and `reject:` commands (after `"$SURT" answer <name> --accept` if the Spec Agent asked first). Exit code `0` (`echo $?`).
- [ ] Read the spec: `cat .surt/subjects/<name>/spec.md`. It describes both scripts.
- [ ] Exercise rejection once: `"$SURT" reject <name> -m "Also say that both scripts exit 2 without an argument"`, then `"$SURT" continue <name>`. The new spec revision mentions exit code 2.
- [ ] `"$SURT" approve <name>` prints `✔ approved spec rev 2 …` and `next: surt continue <name>`.

## 5. The Tasks

- [ ] `"$SURT" continue <name>` runs `plan-tasks` and ends with `⏸ manifest rev 1 awaiting approval …`, exit `0`.
- [ ] `cat .surt/subjects/<name>/manifest.json` lists **exactly two** Tasks (greet and farewell), and `.surt/subjects/<name>/tasks/` holds one file per Task. If the agent produced another split, reject it with `"$SURT" reject <name> -m "Exactly two tasks: one for greet.sh, one for farewell.sh"` and run `"$SURT" continue <name>` again.
- [ ] `"$SURT" approve <name>` prints `next: surt continue <name>`.

## 6. Implementation, interrupted once

This step also checks the process-tree handling on macOS: a graceful stop reaches Claude Code and the whole tree ends within the grace period (`stop_grace_period`, 10 s by default). Run it with no `~/.surt/config.toml`, so the default applies.

- [ ] `"$SURT" continue <name>`. Wait until a line `● item … build started` appears.
- [ ] In a second terminal, `pgrep -fl 'claude -p'` shows at least one Claude Code process.
- [ ] Press **Ctrl-C once** in the first terminal. Within the 10 s grace period `surt` exits with `130` and prints how to resume.
- [ ] `pgrep -fl 'claude -p'` prints nothing: no orphaned agent survived the stop.
- [ ] `"$SURT" continue <name>` resumes. Expected lines, in some order:
  - `● item <key> build started` for both Tasks (Builders run in their own worktrees, in parallel; the keys come from the manifest);
  - reviews of both Tasks; a `changes_requested` review followed by a new build iteration is fine;
  - `✔ item <key> merged (<sha7>)` for both Tasks;
  - finally `✔ delivered — surt/<name> @ <sha7>` and `✔ run succeeded`.
- [ ] Exit code `0`.

If a step blocks (exit `4`), read the printed reason and log path. `unsupported_version` means Claude Code is too old; `authentication_failed` means the subscription login is missing. Fix the cause and run `"$SURT" continue <name>` again. A rate limit does not block: Surt prints `⏸ agent launches paused until <time> — N waiting · Ctrl-C to stop (resumable)` and resumes by itself after the reset.

## 7. Expected final state

- [ ] The run succeeded:

  ```sh
  sqlite3 .surt/surt.db "SELECT state FROM workflow_run WHERE parent_step_run_id IS NULL"
  ```

  prints `succeeded`.
- [ ] Both Tasks merged and the Shipper delivered:

  ```sh
  sqlite3 .surt/surt.db "SELECT w.item_key, s.outcome FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id WHERE s.step_key = 'integrate' AND s.outcome = 'merged'"
  sqlite3 .surt/surt.db "SELECT outcome FROM step_run WHERE step_key = 'ship'"
  ```

  print two Tasks with `merged`, then `done`.
- [ ] The Integration Branch holds the work and passes the tests:

  ```sh
  git log --oneline main..surt/<name>
  git worktree add --detach ../toy-check surt/<name>
  (cd ../toy-check && sh test.sh && sh greet.sh Ada && sh farewell.sh Ada)
  git worktree remove ../toy-check
  ```

  The log shows both Tasks' commits and their merges, the tests print `all tests passed`, and the scripts print `Hello, Ada!` and `Goodbye, Ada!`.
- [ ] `main` did not move (`merge_into_base = "never"`): `git rev-parse main` equals the `<sha7>` printed by `surt run`.
- [ ] Every agent Execution recorded the Claude Code version:

  ```sh
  sqlite3 .surt/surt.db "SELECT DISTINCT json_extract(runtime_info, '$.runtime_version') FROM execution WHERE kind = 'agent'"
  ```

  prints the version `claude --version` shows, and nothing else (no empty row).
- [ ] Every agent Execution has its logs: `ls .surt/logs/<name>/` shows one directory per Execution, each with a non-empty `stream.jsonl` and a `stderr.log`.
- [ ] `pgrep -fl 'claude -p'` prints nothing.

## 8. Record the result

Add a line to the "Smoke results" section of ticket 22 (`.scratch/milestone-1/issues/22-real-claude-code-in-the-binary-and-macos-smoke.md`) and tick its last checkbox when every box above is ticked:

```
- YYYY-MM-DD · macOS <version> (<arch>) · Claude Code <version> · surt <commit> · <pass | fail: step and reason>
```

A failure is still a result: record it with the step that failed and the log path, and open a ticket for it.

# Milestone 2: operate and debug a run with the real Claude Code

This part makes "Milestone 2 done" a repeatable check ([Milestone 2 spec](../.scratch/milestone-2/spec.md), "Done when" criterion 2). It covers what only a real terminal and the real `claude` show: the footer redraws during `surt continue`, `surt logs -f` renders a live Builder, `surt stop` from a second terminal leaves a resumable run, and `surt inspect <execution>` shows session, model, turns and cost. Everything else in Milestone 2 is covered by the automated suite.

It reuses sections 1–3 above (prerequisites, release binary, toy repository) and spends about as much of your subscription as the Milestone 1 run. Use a **fresh** toy repository: run section 3 again. Work in a terminal window at least 100 columns wide, with a UTF-8 locale (`echo $LANG` names `UTF-8`).

## M2.1 Reach the implementation

- [ ] Create, approve and plan a Feature as in sections 4 and 5, without the rejection step:

  ```sh
  "$SURT" run feature --input request="Add greet.sh, which prints 'Hello, <name>!' for its first argument, and farewell.sh, which prints 'Goodbye, <name>!'. Each script gets its own test under tests/."
  "$SURT" approve <name>
  "$SURT" continue <name>
  "$SURT" approve <name>
  ```

  The manifest lists two Tasks; reject and continue as in section 5 if it does not.

## M2.2 The footer redraws during `surt continue`

- [ ] Terminal 1: `"$SURT" continue <name>`.
- [ ] Under the event lines a single footer line is redrawn in place about once a second, for example `── 2 running (A build 42s, B build 41s) · slots 2/3 · 1m ──`. Durations grow; the line never scrolls or repeats.
- [ ] When an event line arrives (`● item … started`, `✔ …`), it prints above the footer and the footer is redrawn below it. Scrolling back shows only event lines, no stale footers.
- [ ] Leave terminal 1 running for the next two sections.

## M2.3 `surt logs -f` renders a live Builder

- [ ] Terminal 2, in the toy repository, while a Builder runs: `"$SURT" logs <name> -f`.
- [ ] Lines appear as the agent works, prefixed with the source when more than one prints, e.g. `A build#1 │ Read greet.sh`, `B build#1 │ Write tests/farewell.sh`, an assistant message's first line in quotes. No raw JSON is printed.
- [ ] Find the Execution id of one running Builder (it is the directory name under `.surt/logs/<name>/`; `ls -t .surt/logs/<name>/ | head -1`), and in a third terminal follow only it: `"$SURT" logs <execution-id> -f` prints that Builder's lines without a prefix.
- [ ] Leave `logs -f` in terminal 2 running.

## M2.4 `surt stop` from a second terminal leaves a resumable run

- [ ] Terminal 3, while at least one Builder runs: `"$SURT" stop <name>`. It prints `⏳ stop requested; driver pid … is stopping N executions…`, then, within the 10 s default grace period, `✔ stopped. resumable: surt continue <name>`, and exits `0` (`echo $?`).
- [ ] Terminal 1 prints `⏳ stop requested from another terminal; stopping…`, one `⏹ item … attempt N interrupted` line per running Execution, then `stopped by surt stop (from another terminal)` with `resume: surt continue <name>`, erases the footer, and exits `130` (`echo $?`).
- [ ] Terminal 2: `logs -f` drains the remaining lines and exits `0` on its own once the Driver is gone.
- [ ] `pgrep -fl 'claude -p'` prints nothing.
- [ ] `"$SURT" status <name>` shows the run `INTERRUPTED` with no Driver, and the interrupted Executions spent no Attempt (each Task's attempt count is unchanged).
- [ ] Terminal 1: `"$SURT" continue <name>` resumes and the run reaches `✔ run succeeded`, exit `0`, as in section 6.

## M2.5 `surt inspect <execution>` shows session, model, turns and cost

- [ ] `"$SURT" inspect <name> A` prints item A's timeline, ending with its `execution.succeeded` and `step.done` events; copy the Execution id from its `execution.started` line for the build step.
- [ ] `"$SURT" inspect <execution-id>` prints, among the other lines:
  - `session   <uuid> · model <model> · version <claude --version>`;
  - `usage     <N> turns · … input · … output · $<cost>` with a non-zero turn count and cost;
  - `logs      surt logs <execution-id>`.
- [ ] `"$SURT" inspect <first 8 characters of the id>` prints the same Execution, or refuses with exit `3` and lists candidates when the prefix is shared.
- [ ] `"$SURT" inspect <execution-id> --json | jq '.execution.reported'` shows `session_id`, `model`, `turns` and `cost_usd`.

## M2.6 Record the result

Add a line to the "Smoke results" section of ticket 16 (`.scratch/milestone-2/issues/16-m2-close-out-and-macos-smoke.md`) and tick its manual-check box when every box of M2.1–M2.5 is ticked:

```
- YYYY-MM-DD · macOS <version> (<arch>) · Claude Code <version> · surt <commit> · <pass | fail: step and reason>
```

A failure is still a result: record it with the step that failed and the command's output, and open a ticket for it.

# Milestone 3: run unattended with the real Claude Code

This part makes "Milestone 3 done" a repeatable check ([Milestone 3 spec](../.scratch/milestone-3/spec.md), story 48). It covers what only the real `claude` and several real terminals show: two Features of one project driven at once share `max_concurrent_agents`, and `stop_grace_period` in the global `~/.surt/config.toml` governs `surt stop`. A real rate limit cannot be provoked on demand, so the Transient Failure pause is covered only by the automated suite (the Fake Runtime and the stream fixtures).

It reuses sections 1–3 above and spends about twice the Milestone 1 run: two Features, each with two Tasks. Use a **fresh** toy repository: run section 3 again. Before starting, `ls ~/.surt/config.toml` should fail (no global file yet); if one exists, move it aside and put it back at the end.

## M3.1 One agent slot for the project

- [ ] Lower the limit and commit it:

  ```sh
  sed -i '' 's/^max_concurrent_agents = 3/max_concurrent_agents = 1/' .surt/config.toml
  grep max_concurrent_agents .surt/config.toml
  git commit --quiet -am "surt: one agent at a time"
  ```

  `grep` prints `max_concurrent_agents = 1 …`.
- [ ] Create, approve and plan two Features as in sections 4 and 5, without the rejection step. Call the first `<one>` and the second `<two>`:

  ```sh
  "$SURT" run feature --input request="Add greet.sh, which prints 'Hello, <name>!' for its first argument, and farewell.sh, which prints 'Goodbye, <name>!'. Each script gets its own test under tests/."
  "$SURT" approve <one>
  "$SURT" continue <one>
  "$SURT" approve <one>
  "$SURT" run feature --input request="Add shout.sh, which prints its first argument in upper case, and whisper.sh, which prints it in lower case. Each script gets its own test under tests/."
  "$SURT" approve <two>
  "$SURT" continue <two>
  "$SURT" approve <two>
  ```

  Each manifest lists two Tasks; reject and continue as in section 5 if one does not.

## M3.2 Two Features never run two Builders

- [ ] Terminal 1: `"$SURT" continue <one>`. Terminal 2, a few seconds later: `"$SURT" continue <two>`.
- [ ] Terminal 3, in the toy repository, while both run: `while sleep 5; do clear; "$SURT" status; pgrep -fl 'claude -p'; done`. For as long as both Features show `RUNNING`:
  - the last line of `status` reads `agent slots: 1/1 in use` or `agent slots: 0/1 in use`, never `2/1`;
  - `pgrep` lists at most one `claude -p` process.
- [ ] While an agent of `<one>` runs, stop the loop (Ctrl-C) and run `"$SURT" status <two>`: an item of `<two>` reads `⏳ queued … waiting for agent slot (1/1 in use)`, although `<two>` itself runs nothing. Restart the loop.
- [ ] The footers in terminals 1 and 2 show `slots 1/1` whenever either Feature has an agent running, not only their own.
- [ ] Both Features reach `✔ run succeeded`, exit `0` (`echo $?` in each terminal).
- [ ] Stop the loop in terminal 3. `pgrep -fl 'claude -p'` prints nothing.

## M3.3 An invalid global file refuses every command

- [ ] `mkdir -p ~/.surt && printf 'stop_grace_period = "ten"\n' > ~/.surt/config.toml`
- [ ] `"$SURT" status` refuses and exits `3` (`echo $?`), naming the file, the line and a reason that mentions `stop_grace_period`:

  ```
  ✖ invalid configuration in /Users/<you>/.surt/config.toml:1
    … stop_grace_period …
    fix: edit /Users/<you>/.surt/config.toml:1
  ```

## M3.4 `stop_grace_period = "30s"` and a `surt stop` that lets a Builder exit

- [ ] `printf 'stop_grace_period = "30s"\n' > ~/.surt/config.toml`, then `"$SURT" status` works again, exit `0`.
- [ ] Raise the limit so both Builders of the next Feature run at once:

  ```sh
  sed -i '' 's/^max_concurrent_agents = 1/max_concurrent_agents = 2/' .surt/config.toml
  git commit --quiet -am "surt: two agents at a time"
  ```

- [ ] Create, approve and plan a third Feature `<three>` as in M3.1, for example "Add count.sh, which prints the number of its arguments, and first.sh, which prints its first argument. Each script gets its own test under tests/."
- [ ] Terminal 1: `"$SURT" continue <three>`. Wait until a `● item … build started` line appears and `pgrep -fl 'claude -p'` lists the Builder.
- [ ] Terminal 2: `"$SURT" stop <three>`. It prints `⏳ stop requested; driver pid … is stopping N executions…`, then, within the 30 s grace period, `✔ stopped. resumable: surt continue <three>`, and exits `0`.
- [ ] Terminal 1 prints one `⏹ item … attempt N interrupted` line per running Builder, then `resume: surt continue <three>`, and exits `130`.
- [ ] `pgrep -fl 'claude -p'` prints nothing: every Builder exited.
- [ ] `"$SURT" inspect <three>` shows each stopped Builder's `execution.interrupted`, and `"$SURT" status <three>` shows the run `INTERRUPTED` with no Driver and no Attempt spent.
- [ ] Terminal 1: `"$SURT" continue <three>` resumes and reaches `✔ run succeeded`, exit `0`.
- [ ] Remove the global file (`rm ~/.surt/config.toml`), or put back the one you moved aside.

That a Builder ignoring the stop is force-killed only after the configured period is covered by the stub-process tests; the real Claude Code exits on the graceful stop.

## M3.5 Record the result

Add a line to the "Smoke results" section of ticket 07 (`.scratch/milestone-3/issues/07-m3-close-out-and-macos-smoke.md`) and tick its manual-check box when every box of M3.1–M3.4 is ticked:

```
- YYYY-MM-DD · macOS <version> (<arch>) · Claude Code <version> · surt <commit> · <pass | fail: step and reason>
```

A failure is still a result: record it with the step that failed and the command's output, and open a ticket for it.

# Milestone 4: fit the project with the real Claude Code

This part makes "Milestone 4 done" a repeatable check ([Milestone 4 spec](../.scratch/milestone-4/spec.md), "Shipping"). It covers what only the real `claude` and a real checked-out `main` show: a Builder on another model that reaches Claude Code as that model, a fast-forward of `main` inside the user's own worktree, and a successful run whose worktrees stay until `surt delete`. Invalid Agent files, every row of the fast-forward table and the interrupted fast-forward are covered by the automated suite with the Fake Runtime and real Git.

It reuses sections 1–3 above and spends about two Milestone 1 runs: two Features, without the rejection step. Use a **fresh** toy repository: run section 3 again. Choose a model your subscription can use other than the Template's `claude-sonnet-5-5`; the steps below say `claude-opus-5-5`.

Since Milestone 5 there are no overrides: `.surt/agents/builder/agent.toml` is the Builder's whole definition, written by `surt init --template feature`, and M4.1 edits it in place.

## M4.1 The Builder on another model

- [ ] Change the Builder's model and commit it, so `main` stays clean:

  ```sh
  sed -i '' 's/^model = "claude-sonnet-5-5"$/model = "claude-opus-5-5"/' .surt/agents/builder/agent.toml
  grep '^model' .surt/agents/builder/agent.toml
  git commit --quiet -am "surt: builder on another model"
  git status --short
  ```

  `grep` prints `model = "claude-opus-5-5"`; `git status --short` prints nothing.

## M4.2 `surt run feature --merge` fast-forwards a checked-out, clean `main`

- [ ] `git branch --show-current` prints `main`: the toy repository's own worktree holds the Base branch.
- [ ] Create the Feature with the per-run flag. Call it `<one>`:

  ```sh
  "$SURT" run feature --merge --input request="Add greet.sh, which prints 'Hello, <name>!' for its first argument, with its own test under tests/."
  ```

  Under `✔ created <one>   workflow: feature   base: main @ <base7>` it prints `merge into base: ff-only (--merge)`. Exit `0`.
- [ ] Approve and continue as in sections 4 and 5, without the rejection step: `"$SURT" approve <one>`, `"$SURT" continue <one>`, `"$SURT" approve <one>`.
- [ ] `"$SURT" continue <one>` ends with `✔ delivered — surt/<one> @ <tip7>`, then `✔ merged into main (fast-forward <base7> → <tip7>)`, then `✔ run succeeded`. Exit `0`.
- [ ] `main` is at the delivered tip and its files moved with it:

  ```sh
  git rev-parse --short main surt/<one>
  git status --short
  sh test.sh && sh greet.sh Ada
  git reflog -1 main
  ```

  Both SHAs are `<tip7>`; `git status --short` prints nothing; the tests print `all tests passed` and the script `Hello, Ada!`; the reflog entry reads `surt: fast-forward main to surt/<one>`.
- [ ] `"$SURT" status <one>` shows `merge into base: ff-only` on the `branch` line and `merge    merged into main <base7> → <tip7>`.
- [ ] `"$SURT" inspect <one> <item key>` lists the build step's `execution.started`; copy its Execution id. `"$SURT" inspect <execution-id>` shows `profile   claude-code · claude-opus-5-5 · …` and `session   <uuid> · model claude-opus-5-5 · …`: the edited model reached Claude Code.
- [ ] The Reviewer kept the Template's model: `"$SURT" inspect <execution-id of a review>` shows `model claude-sonnet-5-5`.

## M4.3 `on_success = "keep"` keeps a succeeded run until `surt delete`

- [ ] Keep successful runs and commit it:

  ```sh
  printf '\n[cleanup]\non_success = "keep"\n' >> .surt/config.toml
  git commit --quiet -am "surt: keep successful runs"
  ```

- [ ] Create, approve, continue and approve a second Feature `<two>` as in M4.2, without `--merge`, for example "Add shout.sh, which prints its first argument in upper case, and whisper.sh, which prints it in lower case. Each script gets its own test under tests/." The creation line has no `merge into base:` line.
- [ ] `"$SURT" continue <two>` ends with `✔ run succeeded`, then `· cleanup kept (on_success = "keep") — remove with: surt delete <two>` instead of a `↻ cleanup` line. Exit `0`. `main` did not move (`merge_into_base = "never"`).
- [ ] The worktrees are still there: `git worktree list` lists one worktree per Task under `~/.surt/worktrees/<project-id>/<two>/`, and `git branch --list 'surt/item/<two>/*'` lists their Item Branches.
- [ ] `"$SURT" status` shows `<two>` as `SUCCEEDED` with `· kept`, and `"$SURT" status <two>` ends with `cleanup  kept (on_success = "keep") — remove with: surt delete <two>`.
- [ ] `"$SURT" continue <two>` again refuses with exit `3` (`<two> has no active run …`, suggesting `surt continue <two> --new-run`) and removes nothing: `git worktree list` is unchanged.
- [ ] `"$SURT" delete <two> --yes` exits `0`. Afterwards `git worktree list` lists only the toy repository itself, `git branch --list 'surt/item/<two>/*'` prints nothing, and `git branch --list surt/<two>` still lists the Integration Branch of the succeeded run.
- [ ] `pgrep -fl 'claude -p'` prints nothing.

## M4.4 Record the result

Add a line to the "Smoke results" section of ticket 10 (`.scratch/milestone-4/issues/10-m4-close-out-and-macos-smoke.md`) and tick its manual-check box when every box of M4.1–M4.3 is ticked:

```
- YYYY-MM-DD · macOS <version> (<arch>) · Claude Code <version> · surt <commit> · <pass | fail: step and reason>
```

A failure is still a result: record it with the step that failed and the command's output, and open a ticket for it.

# Milestone 5: the project owns its workflows, with the real Claude Code

This part makes "Milestone 5 done" a repeatable check ([Milestone 5 spec](../.scratch/milestone-5/spec.md), [ADR 0013](adr/0013-definitions-live-in-the-project.md)). It covers what only the real `claude` shows: the feature Template written into a project, an edited Reviewer instruction that the real Reviewer follows and that the run froze, `surt run`/`surt approve`/`surt continue` to the end, a rerun of `surt init --template feature` that keeps edited files, and a workflow of the project's own with no inputs. Invalid definitions, input refusals, `--new-run` with carried inputs and the frozen delivery checks are covered by the automated suite.

It reuses sections 1–2 above and spends about one Milestone 1 run plus one short agent Execution. Use a **fresh** toy repository: run section 3 again, which is where `surt init --template feature` is checked.

## M5.1 The Template is files of the project

- [ ] In the toy repository after section 3, `ls .surt/workflows .surt/agents` lists `feature.toml` and `task-cycle.toml`, and the directories `builder`, `reviewer`, `shipper`, `spec` and `tasks`, each with `agent.toml` and `instructions.md`.
- [ ] `git ls-files .surt` lists `.surt/.gitignore`, `.surt/config.toml`, both workflows and every file under `.surt/agents/` (schemas included), and no `surt.db`: the definitions are versioned with the project.
- [ ] `diff .surt/agents/reviewer/instructions.md <surt checkout>/crates/surt/assets/templates/feature/agents/reviewer/instructions.md` prints nothing: the file is exactly what the binary embeds.

## M5.2 An edited Reviewer instruction reaches the Reviewer

- [ ] Give the Reviewer a project convention and commit it:

  ```sh
  printf '\n## Project convention\n\nStart the `summary` of every review with the word SMOKE-REVIEWED.\n' >> .surt/agents/reviewer/instructions.md
  git commit --quiet -am "surt: reviewer convention"
  ```

- [ ] Create the Feature. Call it `<name>`:

  ```sh
  "$SURT" run feature --input request="Add greet.sh, which prints 'Hello, <name>!' for its first argument, with its own test under tests/."
  ```

  It prints `✔ created <name>   workflow: feature   base: main @ <sha7>`, with no `local agents:` line, then stops at `⏸ spec rev 1 awaiting approval …`. Exit `0`.
- [ ] Edit the file again **after** creation, without committing: `printf 'End every summary with the word NOT-FROZEN.\n' >> .surt/agents/reviewer/instructions.md`. The run froze its Agents when it was created, so this line must never reach its Reviewer.
- [ ] `"$SURT" approve <name>` prints `next: surt continue <name>`; `"$SURT" continue <name>` stops at `⏸ manifest rev 1 awaiting approval …`; `"$SURT" approve <name>` prints `next: surt continue <name>` again.
- [ ] `"$SURT" continue <name>` runs to the end: `✔ item <key> merged (<sha7>)` for each Task, `✔ delivered — surt/<name> @ <tip7>`, `✔ run succeeded`, a `↻ cleanup` line, exit `0`.
- [ ] `"$SURT" inspect <name> <item key>` lists the review step's `execution.started`; copy its Execution id. `"$SURT" inspect <execution-id>` shows the role `reviewer` and the profile `claude-code · claude-sonnet-5-5 · …`.
- [ ] The Reviewer's frozen instructions hold the committed convention and not the later edit:

  ```sh
  sqlite3 .surt/surt.db "SELECT CAST(b.bytes AS TEXT) FROM execution e JOIN snapshot_file f ON f.snapshot_sha256 = e.agent_snapshot_sha256 JOIN blob b ON b.sha256 = f.blob_sha256 WHERE e.id = '<execution-id>' AND f.rel_path = 'instructions.md'" | tail -3
  ```

  prints the line ``Start the `summary` of every review with the word SMOKE-REVIEWED.`` last, and no `NOT-FROZEN`.
- [ ] The Reviewer followed it: `"$SURT" show <name> feedback --item <item key>` prints a `summary` that starts with `SMOKE-REVIEWED` and contains no `NOT-FROZEN`.
- [ ] Discard the uncommitted edit: `git checkout .surt/agents/reviewer/instructions.md`, then `git status --short` prints nothing.

## M5.3 `surt init --template feature` again keeps the project's files

- [ ] Delete one Template file: `rm .surt/agents/shipper/instructions.md`.
- [ ] `"$SURT" init --template feature` prints `✔ surt is already initialized in …/.surt`, then `created` on the line of `.surt/agents/shipper/instructions.md` and `kept` on every other line, `.surt/agents/reviewer/instructions.md` included. Exit `0`.
- [ ] `grep SMOKE-REVIEWED .surt/agents/reviewer/instructions.md` still prints the convention, and `git status --short` prints nothing: the restored Shipper file is the committed one, and the edited Reviewer file was not overwritten.

## M5.4 A workflow of the project's own, with no inputs

- [ ] Add a one-step workflow and its Agent:

  ```sh
  mkdir -p .surt/agents/greeter
  cat > .surt/workflows/hello.toml <<'TOML'
  schema = 1
  name = "hello"

  [steps.greet]
  kind = "agent"
  agent = "greeter"
  TOML
  cat > .surt/agents/greeter/agent.toml <<'TOML'
  schema = 1
  id = "greeter"

  [outputs.note]
  format = "text"

  [profile]
  runtime = "claude-code"
  model = "claude-sonnet-5-5"
  effort = "high"

  [profile.claude]
  auth = "subscription"
  permission_mode = "dontAsk"
  tools = ["Read", "Write"]
  allowed_tools = ["Read"]
  disallowed_tools = ["mcp__*"]
  TOML
  printf '# Greeter\n\nWrite note.txt in the delivery directory named in your context, holding one line that greets the reader. Finish with outcome done and outputs.note set to note.txt.\n' > .surt/agents/greeter/instructions.md
  git add .surt && git commit --quiet -m "surt: hello workflow"
  ```

- [ ] `"$SURT" run hello --input request=x` refuses with exit `3`: `✖ workflow hello has no input "request"`, `workflow hello takes no inputs`, and a `fix:` line. Nothing is created (`"$SURT" status --all` lists no `hello` Subject).
- [ ] `"$SURT" run hello` prints `✔ created <date>-hello   workflow: hello` (named after the workflow, no `base:` part), `● greet started`, `✔ note rev 1 published`, `✔ greet done`, then `✔ run succeeded` with no `delivered` line, since the workflow has no Integration Branch. Exit `0`.
- [ ] `"$SURT" show <date>-hello note` prints the greeting.
- [ ] `pgrep -fl 'claude -p'` prints nothing.

## M5.5 Record the result

Add a line to the "Smoke results" section of ticket 07 (`.scratch/milestone-5/issues/07-m5-close-out.md`) and tick its manual-check box when every box of M5.1–M5.4 is ticked:

```
- YYYY-MM-DD · macOS <version> (<arch>) · Claude Code <version> · surt <commit> · <pass | fail: step and reason>
```

A failure is still a result: record it with the step that failed and the command's output, and open a ticket for it.

# Milestone 6: talk the spec through with the real Claude Code

This part makes "Milestone 6 done" a repeatable check ([Milestone 6 spec](../.scratch/milestone-6/spec.md), story 28, [ADR 0014](adr/0014-a-conversation-continues-across-a-loop.md)). It covers what only the real `claude` shows: a Spec Agent that asks before it writes, answers typed on a terminal, a conversation that continues under a new session id each round, a rejection that continues it, and a restart after its transcript is deleted from `~/.claude/projects/`. The `--file` and `-m` refusals, `status --json`, the delta context's exact Inputs and every rule of the `questions` format are covered by the automated suite with the Fake Runtime and the stub `claude`.

It reuses sections 1–2 above and spends about five Spec Agent Executions and nothing else: the run stops at the approved spec. Use a **fresh** toy repository: run section 3 again. M6.2 needs an interactive terminal, since `surt answer` without a flag reads the answers from it.

## M6.1 The Spec Agent asks

- [ ] Create the Feature with a request vague enough to leave decisions open. Call it `<name>`:

  ```sh
  "$SURT" run feature --input request="Add a script that greets people."
  ```

  It prints `✔ created <name>   workflow: feature   base: main @ <sha7>`, `● spec started`, `✔ spec-questions rev 1 published`, then a block

  ```
  ? spec-questions rev 1 awaiting answers (round 1)
    1. <question>
       why: <what the answer changes in the spec>
       recommended: <answer>
    …
    answer: surt answer <name>
  ```

  with 1 to 5 numbered questions, each with `why:` and `recommended:`, and `options:` (which include the recommendation) when the answer is a closed set. Exit `0`. If it stops at `⏸ spec rev 1 awaiting approval …` instead, the Agent found nothing to ask: `"$SURT" cancel <name> --yes` and run this step again with a vaguer request.
- [ ] The questions are worth asking: none is answered by the toy repository itself (`README.md`, `test.sh`, `tests/`), and each one changes what the script does or how it is tested (its name, its message, what happens without an argument).
- [ ] `"$SURT" status <name>` shows `AWAITING ANSWER`, the line `spec-questions rev 1 (agent) ⏸ awaiting answers (round 1)` and `answer: surt answer <name>`.
- [ ] `"$SURT" approve <name>` refuses with exit `3`: `✖ <name> has no approval gate waiting`, `answer-spec is waiting for answers, not for approval`, `next: surt answer <name>`.

## M6.2 Answer on the terminal; the conversation continues

- [ ] `"$SURT" answer <name>` prints the first question with its `why:` line and its recommendation in brackets, and waits. Press Enter on the first question to accept its recommendation, and type an answer of your own on the second (for example `greet.sh, printing 'Hi, <name>!'`), then Enter on any other. With a single question, type your own answer on it.
- [ ] It prints `✔ answered spec-questions rev 1 by <actor>`, then `● spec continued (iteration 2, same conversation)`, not `● spec started`, and stops either at `? spec-questions rev 2 awaiting answers (round 2)` or at `⏸ spec rev 1 awaiting approval in .surt/subjects/<name>`. Exit `0`.
- [ ] `"$SURT" show <name> spec-answers` prints one round: the questions of rev 1, the first question's recommendation as its answer and your text as the second's.
- [ ] `"$SURT" inspect <name>` lists an `execution.started` line for `spec iter 1` and one for `spec iter 2`; copy both Execution ids. `"$SURT" inspect <iteration 1 execution-id>` shows `conversation  fresh: no_previous`. `"$SURT" inspect <iteration 2 execution-id>` shows `conversation  continued from <iteration 1 execution-id>`, and its `session` line names a session id different from the first one's: every Execution keeps its own session.

## M6.3 A second round, or the spec

- [ ] If a second round waits: `"$SURT" status <name>` shows `spec-questions rev 1 (agent) ✔ answered by <actor> … (round 1)` and `spec-questions rev 2 (agent) ⏸ awaiting answers (round 2)`. `"$SURT" answer <name> -m "Decide the rest yourself."` prints `✔ answered spec-questions rev 2 by <actor>` and `● spec continued (iteration 3, same conversation)`, then stops at the spec gate (a further round: `"$SURT" answer <name> --accept`). `"$SURT" show <name> spec-answers` prints both rounds, the second with the `note`.
- [ ] If the Spec Agent wrote the spec after one round, the result line of M6.6 says `one round`.
- [ ] `cat .surt/subjects/<name>/spec.md` has a **Decisions** section with each question asked and the answer given, your typed answer included, and no **Open questions** section.

## M6.4 A rejection continues the conversation

- [ ] `"$SURT" reject <name> -m "Also say that the script exits 2 without an argument"`, then `"$SURT" continue <name>`. It prints `● spec continued (iteration N, same conversation)` and stops at `⏸ spec rev 2 awaiting approval …` (answer with `--accept` if it asks first). Exit `0`.
- [ ] `cat .surt/subjects/<name>/spec.md` mentions exit code `2` and still lists the **Decisions**.
- [ ] `"$SURT" inspect <name>` lists the new `spec iter N` Execution; `"$SURT" inspect <its execution-id>` shows `conversation  continued from <the previous spec Execution's id>`.

## M6.5 A lost transcript restarts the conversation

- [ ] Take the latest spec Execution, the one that wrote spec rev 2 in M6.4, and its session id from the `session` line of `"$SURT" inspect <its execution-id>`. It is the conversation the next spec Execution would continue.
- [ ] Delete its transcript:

  ```sh
  ls ~/.claude/projects/*/<session-id>.jsonl
  rm ~/.claude/projects/*/<session-id>.jsonl
  ```

  `ls` lists exactly one file, in the directory Claude Code named after the toy repository's path (for example `~/.claude/projects/-Users-<you>-surt-smoke-toy/`).
- [ ] `"$SURT" reject <name> -m "Name the test file tests/greet.sh"`, then `"$SURT" continue <name>`. It prints `● spec continued (iteration M, same conversation)`, then `↻ spec restarted (iteration M): transcript_missing`, and stops at `⏸ spec rev 3 awaiting approval …`. Exit `0`. There is no `↻ spec iter M attempt 2` line: the restart spent no Attempt.
- [ ] `"$SURT" inspect <name>` lists the `spec iter M` Execution; `"$SURT" inspect <its execution-id>` shows `conversation  restarted: transcript_missing (previous <the execution-id that wrote spec rev 2>)`, and its `state` line says `succeeded`.
- [ ] The restarted Spec Agent read every Input again: `spec.md` names `tests/greet.sh`, still says exit code `2` and still lists the **Decisions** of every round.

## M6.6 Approve, and record the result

- [ ] `"$SURT" approve <name>` prints `✔ approved spec rev 3 …` and `next: surt continue <name>`. The rest of the run is what sections 5–7 check; to spend nothing more, `"$SURT" cancel <name> --yes`.
- [ ] `pgrep -fl 'claude -p'` prints nothing.

Add a line to the "Smoke results" section of ticket 08 (`.scratch/milestone-6/issues/08-m6-close-out.md`) and tick its manual-check box when every box of M6.1–M6.6 is ticked:

```
- YYYY-MM-DD · macOS <version> (<arch>) · Claude Code <version> · surt <commit> · <rounds of questions> · <pass | fail: step and reason>
```

A failure is still a result: record it with the step that failed and the command's output, and open a ticket for it.
