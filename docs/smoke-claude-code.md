# Manual smoke: the real Claude Code on macOS

This checklist makes "Milestone 1 done" a repeatable check ([K. Roadmap](architecture/K-roadmap.md), criterion 2): in a toy repository, a Feature with two Tasks reaches `succeeded` with the real Claude Code. Everything else in Milestone 1 is covered by the automated suite with the Fake Runtime and stub executables; this is the only check that runs the real `claude`.

**It spends your Claude subscription.** A full run launches about seven agent Executions (spec, tasks, two Builders, two Reviewers, the Shipper), more if a review asks for changes. Run it deliberately, not in a loop.

Work through the boxes in order. Every command runs in the toy repository unless stated otherwise. `<name>` is the Feature name `surt feature` prints (for example `20261018-greet-and-farewell`).

## 1. Prerequisites

- [ ] macOS with Git 2.38 or newer: `git --version`.
- [ ] Claude Code **2.1.301 or newer** on `PATH`: `claude --version` prints e.g. `2.1.301 (Claude Code)`. Upgrade with `claude update`.
- [ ] Claude Code is logged in with a **subscription** (run `claude` once interactively and `/login` if needed). Surt never passes API keys, gateways or `ANTHROPIC_*` variables through, so an API-key-only setup fails with `authentication_failed`.
- [ ] The subscription can use the model of the built-in agents, `claude-sonnet-5-5` (see `crates/surt/assets/agents/*/agent.toml`).
- [ ] No other `claude -p` process is running: `pgrep -fl 'claude -p'` prints nothing. Later steps use this to detect leftovers.

## 2. Build the release binary

From the Surt checkout:

- [ ] `cargo build --release -p surt` succeeds. Do **not** pass `--features test-support`: the release binary has no Fake Runtime and always uses Claude Code.
- [ ] `export SURT="$PWD/target/release/surt"` and `"$SURT" --help` lists `init`, `feature`, `plan`, `start`, `approve`, `reject`.
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

- [ ] `"$SURT" init` prints a success line and creates `.surt/config.toml` and `.surt/.gitignore`.
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

- [ ] `git status --short` is empty: `.surt/.gitignore` keeps the runtime state out of Git.

## 4. Spec phase

- [ ] Create the Feature:

  ```sh
  "$SURT" feature "Add greet.sh, which prints 'Hello, <name>!' for its first argument, and farewell.sh, which prints 'Goodbye, <name>!'. Each script gets its own test under tests/."
  ```

  Expected: `✔ created <name>   base: main @ <sha7>`, then `● spec started`, then the spec waiting for approval with the next command. Exit code `0` (`echo $?`).
- [ ] Read the spec: `cat .surt/features/<name>/spec.md`. It describes both scripts.
- [ ] Exercise rejection once: `"$SURT" reject <name> -m "Also say that both scripts exit 2 without an argument"`, then `"$SURT" feature <name>`. The new spec revision mentions exit code 2.
- [ ] `"$SURT" approve <name>` prints `✔ approved spec rev 2 …` and `next: surt plan <name>`.

## 5. Task phase

- [ ] `"$SURT" plan <name>` ends waiting on the task manifest, exit `0`.
- [ ] `cat .surt/features/<name>/manifest.json` lists **exactly two** Tasks (greet and farewell), and `.surt/features/<name>/tasks/` holds one file per Task. If the agent produced another split, reject it with `"$SURT" reject <name> -m "Exactly two tasks: one for greet.sh, one for farewell.sh"` and run `"$SURT" plan <name>` again.
- [ ] `"$SURT" approve <name>` prints `next: surt start <name>`.

## 6. Implementation, interrupted once

This step also checks the process-tree handling on macOS: a graceful stop reaches Claude Code and the whole tree ends within the 10 s grace period.

- [ ] `"$SURT" start <name>`. Wait until a line `● task … build started` appears.
- [ ] In a second terminal, `pgrep -fl 'claude -p'` shows at least one Claude Code process.
- [ ] Press **Ctrl-C once** in the first terminal. Within about 10 s `surt` exits with `130` and prints how to resume.
- [ ] `pgrep -fl 'claude -p'` prints nothing: no orphaned agent survived the stop.
- [ ] `"$SURT" start <name>` resumes. Expected lines, in some order:
  - `● task <key> build started` for both Tasks (Builders run in their own worktrees, in parallel; the keys come from the manifest);
  - reviews of both Tasks; a `changes_requested` review followed by a new build iteration is fine;
  - `✔ task <key> merged (<sha7>)` for both Tasks;
  - finally `✔ shipper delivered — surt/<name> @ <sha7>` and `✔ run succeeded`.
- [ ] Exit code `0`.

If a step blocks (exit `4`), read the printed reason and log path. `unsupported_version` means Claude Code is too old; `authentication_failed` means the subscription login is missing; `rate_limit` prints when it resets. Fix the cause and run `"$SURT" start <name>` again.

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
- [ ] `main` did not move (`merge_into_base = "never"`): `git rev-parse main` equals the `<sha7>` printed by `surt feature`.
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
