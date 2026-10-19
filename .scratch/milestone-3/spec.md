# Spec: Milestone 3 — run unattended

Labels: ready-for-agent
Type: spec
Status: open (5 of 7 tickets resolved; 01 awaits the first macOS/Linux CI run, 07 awaits CI on three OSes and the manual Claude Code smoke; tickets in [issues/](issues/))
Map: [Surt — Architecture and implementation plan](../surt-architecture/map.md)
Plan: [K. Roadmap](../../docs/architecture/K-roadmap.md#after-milestone-1), [I. Recovery — Transient Failures](../../docs/architecture/I-recovery.md#transient-failures-network-and-rate-limits), [A. Process Model — agent concurrency limit](../../docs/architecture/A-process-model.md#agent-concurrency-limit), [J. Configuration](../../docs/architecture/J-cli.md#configuration)
Previous: [Milestone 2](../milestone-2/spec.md)

## Problem Statement

Milestones 1 and 2 deliver a Feature end to end and let the developer understand, stop and clean up any run from the CLI. They still assume someone is watching. A long `surt start` left in a tmux pane, a CI job or overnight fails to finish in three ways:

- **A rate limit ends the night.** When the account hits its usage limit, or the API is overloaded, or the network drops, Claude Code gives up after its own retries. Surt already classifies this as a Transient Failure and spends no Attempt, but Milestone 1 then **blocks** the Step Run at once (`blocked_reason = 'not_retryable'`, exit `4`). The developer comes back in the morning to `⛔ task C build blocked: cannot be retried … — resets 03:00`, a run that has done nothing for five hours, and a command to type that would have worked at 03:01. With three Builders in flight, all three block one after the other.
- **Two Features in one project double the agents.** `max_concurrent_agents` is a per-Driver semaphore. Driving `add-login` in one terminal and `fix-cache` in another runs 3 + 3 Claude sessions against one account and one machine, which is the fastest way to hit the rate limit above. `surt status` only counts the Driver's own agents, so the `agent slots` line it prints is wrong whenever two Drivers run, and a lowered limit in `.surt/config.toml` only applies to Drivers started afterwards.
- **The stop grace period is fixed.** A Builder that runs a slow test suite or a large `cargo build` gets 10 seconds after `SIGTERM` (or Ctrl+Break on Windows) before it is killed, on every `surt stop`, Ctrl-C, `surt cancel` and recovery. There is no place to change it per machine: the global `~/.surt/config.toml` that J describes is never read.

## Solution

Milestone 3 ("Run unattended") ships the three mechanisms K deferred to it. With them, a developer can start a long `surt start` and come back to a finished run, or to a run that is blocked only because a human is really needed.

```
$ surt start add-login
✔ task A merged (3c1aa20)
✖ task B build attempt 2: rate limited (resets 03:00)
⏸ agent launches paused until 03:00 — 2 waiting · Ctrl-C to stop (resumable)
✖ task C review: rate limited (resets 03:00)
↻ agent launches resumed
↻ task B build attempt 2
↻ task C review
✔ task B merged (9e04b7d)
✔ task C merged (a0e1f55)
✔ shipper    delivered — surt/20261017-add-login @ 4e1b2c0
✔ run        succeeded
```

- **A Transient Failure pauses instead of blocking.** Every agent launch of that Driver waits until the reset time the stream reported, or, without one, with a backoff of 1, 2, 4, 8 and then 15 minutes between tries. Running agents go on, Operations (Checks, integrations) keep running, nothing is spent, and there is no ceiling. Ctrl-C or `surt stop` turns the wait into an ordinary resumable Interruption. `surt status` and `surt inspect` show the pause from any terminal.
- **Agent slots are shared by the whole project.** Each slot is a lock file under `.surt/runtime/slots/`. Every Driver of the project takes a slot before launching an agent and gives it back when the agent ends; a Driver that dies frees its slots through the OS. `max_concurrent_agents` is read before every launch, so lowering it takes effect without restarting anything. `surt status` prints `agent slots: N/M in use` for the project, counting every live Driver.
- **`~/.surt/config.toml` is read.** On Windows it is `%USERPROFILE%\.surt\config.toml`. It holds `stop_grace_period` (default `10s`). Unknown keys and bad values refuse every command with exit `3`, the same way the project configuration does.

Like Milestones 1 and 2, nothing here needs a migration: the columns (`execution.slot`, `failure_class`, `runtime_info.resets_at`) and event kinds (`agents.paused`, `agents.resumed`) already exist in `0001_initial.sql` and D.

## User Stories

### Rate limits and outages: the pause

1. As a developer leaving `surt start` running overnight, I want a rate-limited agent to wait for the reset instead of blocking, so that the run continues on its own when the limit lifts.
2. As a developer, I want the pause to last until the reset time Claude Code reported, so that Surt neither hammers the API before the reset nor waits longer than it must.
3. As a developer whose failure carries no reset time (an overload, a server error, a network outage), I want Surt to retry with a backoff of 1, 2, 4, 8 and then 15 minutes, so that a short outage costs a minute and a long one does not flood the API.
4. As a developer, I want a Transient Failure to spend no Attempt, so that a night of rate limits does not exhaust `max_attempts` and block a Task that never actually failed.
5. As a developer, I want every agent launch of the Driver to pause when one agent hits a Transient Failure, so that the other Tasks do not each burn through Claude Code's internal retries against the same account limit.
6. As a developer, I want agents already running when the pause starts to go on, so that work in progress is not thrown away because another agent was rate limited.
7. As a developer, I want an agent that was already running and then hits the limit during the pause to fail transient as well and simply wait with the others, so that the pause behaves the same whichever agent noticed first.
8. As a developer, I want Checks and integrations to keep running during a pause, so that a Task whose review already passed is merged while the Builders wait.
9. As a developer, I want the pause to have no ceiling, so that a five-hour usage limit is waited out instead of turning into a block at some arbitrary threshold.
10. As a developer, I want the foreground output to print one line when launches pause, naming the reset time (or the backoff delay) and how many agents are waiting, so that I can tell at a glance that the run is waiting, not hanging.
11. As a developer, I want a line when launches resume, so that the scrollback shows how long the run waited.
12. As a developer watching a TTY, I want the footer to say that launches are paused and until when, so that the live view explains why nothing new starts.
13. As a developer reading CI output, I want the 5-minute heartbeat line to mention the pause, so that a CI log that waits two hours for a reset does not look like a hang.
14. As a developer, I want Ctrl-C during a pause to be an ordinary Interruption (exit `130`, resumable), so that I can stop waiting without losing anything.
15. As a developer, I want `surt stop` from another terminal to interrupt a paused Driver the same way, so that I can stop an overnight run from my laptop's other session.
16. As a developer resuming with `surt start` after an Interruption during a pause, I want the new Driver to try again at once and learn the pause again from the first failure, so that I can resume as soon as I think the limit has lifted.
17. As a developer, I want the backoff to start over once an agent Execution ends in any other way, so that an outage in the afternoon does not make the evening's first transient failure wait 15 minutes.
18. As a developer, I want a reported reset time that is already in the past to be treated as no reset time, so that a skewed clock never makes Surt retry in a tight loop.
19. As a developer running `surt status` in another terminal, I want the run to show that agent launches are paused, why, and until when if known, so that I know the Driver is alive and waiting, not stuck.
20. As a developer running `surt status --json`, I want the pause as a field of the run document, so that a script can tell a paused run from a busy one.
21. As a developer running `surt inspect <feature>`, I want `agents.paused` and `agents.resumed` events on the timeline with the reset time, the error code and the number waiting, so that I can reconstruct a night of rate limits afterwards.
22. As a developer running `surt inspect <execution>` on a rate-limited Execution, I want it to show the class `transient`, the error code and the reset time, so that I can see why the Attempt did not count.
23. As a developer, I want a `not_retryable` failure (expired login, billing error, unknown model) to keep blocking at once, so that Surt never waits forever on something a human must fix.
24. As a developer, I want an unknown error category to stay `retryable` and spend an Attempt, so that a misclassification costs Attempts and never makes Surt wait forever on a permanent error.
25. As a developer with a run that Milestone 1 or 2 blocked on a Transient Failure, I want `surt start` to resume it with the usual fresh budget, so that upgrading the binary needs nothing special.

### Agent slots across processes

26. As a developer driving two Features of one project in two terminals, I want them to share `max_concurrent_agents`, so that the project never runs more agents than I configured.
27. As a developer, I want a Driver that finds every slot taken to queue its agents and retry, so that the second Feature waits for a free slot instead of failing.
28. As a developer, I want a Driver killed with `kill -9` to free its slots at once, so that a crash never leaves the other Feature waiting for slots nobody holds.
29. As a developer, I want a lowered `max_concurrent_agents` in `.surt/config.toml` to apply to the next launch of every running Driver, so that I can throttle a long run without stopping it.
30. As a developer, I want agents already running above a lowered limit to finish, so that lowering the limit never kills work.
31. As a developer, I want a raised limit to be used by running Drivers too, so that freeing capacity speeds up a waiting Feature without restarting it.
32. As a developer who saves an invalid `.surt/config.toml` while a Driver runs, I want the Driver to keep its last valid limit and warn once, so that a typo does not crash an overnight run.
33. As a developer running `surt status`, I want `agent slots: N/M in use` counted across every live Driver of the project, so that the line is true when two Features run.
34. As a developer running `surt status <feature>`, I want a queued Task to say `waiting for agent slot (3/3 in use)` with the project-wide count, so that I understand it waits for the other Feature's agents.
35. As a developer watching the footer, I want `slots N/M` to count the project's slots, not only my Driver's, so that the live view agrees with `status`.
36. As a developer running `surt inspect <execution>`, I want the slot the agent held (`agent-1`), so that I can correlate it across Features.
37. As a developer, I want queued agents of one Driver to go in a deterministic order (manifest order for Tasks), so that the same plan launches in the same order.
38. As a developer on Windows, I want slots to work the same as on macOS and Linux, so that the limit holds on every MVP platform.
39. As a developer running Surt in two different projects, I want each project to keep its own slots, so that one project's agents never wait for another's.

### Global configuration

40. As a developer whose Builders run slow test suites, I want `stop_grace_period` in `~/.surt/config.toml`, so that a stop gives the agent enough time to exit cleanly on my machine.
41. As a developer, I want the grace period to apply to every graceful stop (Ctrl-C, `surt stop`, `surt cancel`, stopping survivors during recovery), so that one setting governs them all.
42. As a developer on Windows, I want the file at `%USERPROFILE%\.surt\config.toml`, so that it sits next to the worktrees Surt already keeps there.
43. As a developer without a global file, I want the defaults, so that the file stays optional.
44. As a developer who misspells a key or writes `stop_grace_period = "ten"`, I want every command to refuse with the file, the line and the reason (exit `3`), so that a typo is caught at once instead of silently ignored.
45. As a developer who puts a project key such as `max_concurrent_agents` in the global file, I want it refused as an unknown key, so that I learn it belongs in `.surt/config.toml`.
46. As a developer, I want the grace period to use the same duration format as a Check's `timeout` (`10s`, `2m`), so that I write durations one way everywhere.

### Milestone close-out

47. As a Surt maintainer, I want the M3 suite to pass in CI on macOS, Linux and Windows, so that the three mechanisms hold on every MVP platform.
48. As a Surt maintainer, I want a short manual smoke with the real Claude Code on macOS covering two Features sharing slots and the global grace period, so that M3 is checked against the real runtime.
49. As a Surt maintainer, I want K, A, I, J, D, B and `CONTEXT.md` updated to match what was built, so that the plan still wins on divergence.
50. As a Surt maintainer, I want the B rule exception for per-Driver slots removed, so that the code and the plan agree again.

## Implementation Decisions

The plan documents ([I. Recovery](../../docs/architecture/I-recovery.md#transient-failures-network-and-rate-limits), [A. Process Model](../../docs/architecture/A-process-model.md#agent-concurrency-limit), [J](../../docs/architecture/J-cli.md#configuration), [G](../../docs/architecture/G-state-machines.md#limits), [B](../../docs/architecture/B-workspace.md)) already decide most of M3, and this spec consolidates them. Points the plan left open are marked **decided here**. The implementation writes them back into the plan document named.

### Scope boundary with M4

- M3 ships the three rows of K's "Mechanisms deferred" table marked M3. `ff-only`, `--merge`/`--no-merge`, `on_success = "keep"` and `.surt/agents/` overrides stay M4, with their Milestone 1 refusals unchanged.
- The pause stays per Driver. Coordinating it across Drivers (other Features, other projects) stays out of scope, as the map says.

### The pause on Transient Failure

- **Classification is unchanged.** The Claude Code adapter already classifies `rate_limit`, `overloaded`, `server_error` and transport errors as `transient` and records the reset time from `rate_limit_event` (only a `rejected` status is a limit) into `runtime_info.resets_at`. M3 changes only what the Driver does with the class.
- **`decide()` no longer blocks on `transient`.** A Step Run whose last Execution failed `transient` stays `running` and gets a new Attempt when launches are allowed. Transient Failures keep not counting toward `max_attempts` (existing rule). `not_retryable` keeps blocking at once.
- **`decide()` takes the pause as input**, as B already sketches: the scheduler context gains `paused_until` and `now`. While `now < paused_until` it emits no agent `Launch` (Operations still run). It stays pure and free of `sqlx`/`git`.
- **The pause schedule is a pure function** of the pause state, the failure just observed and `now`, returning the new `paused_until` ([B. Clock](../../docs/architecture/B-workspace.md): pure functions taking `now`, no trait). Its rules:
  - With a reported reset time later than `now`, the pause ends at that reset time. **Decided here:** when several failures report reset times, the pause ends at the latest one.
  - Without one (or with one already in the past, **decided here**, so a skewed clock never makes Surt retry in a tight loop), the pause ends after the next backoff step: 1, 2, 4, 8, then 15 minutes for every further step.
  - **Decided here:** the backoff step advances once per resume that fails, not once per failure. Failures from agents that were already running when the pause began extend nothing beyond the latest reset; they do not advance the step.
  - The backoff resets to its first step when an agent Execution ends in any way other than `transient` (succeeded, `retryable`, `not_retryable`, interrupted).
  - No ceiling on the number of tries or on the total wait.
- **The pause lives in the Driver's memory**, as I says. A new driving command starts unpaused and learns again from its first failure. No column is added.
- **Decided here: no probe launch.** When the pause ends, launches resume normally up to the free slots. A rate-limited resume costs no Attempt, so a single-probe protocol would add a state without saving anything.
- **The Driver loop waits for the earliest of** a landed flight, its normal tick, a raised stop level, or `paused_until`. Ctrl-C, a closed terminal and `surt stop` reach the paused Driver through the existing `StopSignal`, so the Interruption path is unchanged (exit `130`).
- **Events.** The Driver writes `agents.paused` when launches pause (and again when the end of the pause moves later) and `agents.resumed` when they resume, both on the root run. **Decided here**, to be added to D's per-kind field table: `agents.paused` carries `until` (Unix seconds), `code` (the error code of the failure that caused it), `backoff_step` (absent when the reset time came from the stream) and `waiting` (agent Step Runs ready to launch); `agents.resumed` carries `paused_seconds`. The engine never reads them back (D's rule stands).
- **Progress.** The facade's `Progress` gains the `Paused { until }` variant B already lists, plus a `Resumed` variant, and the `ActivitySnapshot` carries the pause so the footer can show it.
- **What `status` shows (decided here).** The pause is derived on every read, like every other run state: a run with a live Driver whose latest ended agent Execution failed `transient`, with no agent Execution started after it, is shown as paused. The headline stays `RUNNING`; the detail adds one line naming the error code and the reset time when the Execution recorded one, or `retrying with backoff` otherwise. The reset time comes from `runtime_info.resets_at`, which `status` already reads for M1's blocked line. `status --json` adds a `paused` object (`code`, `resets_at` when known) to the run document, additively.
- **Upgrading from M1/M2.** A Step Run that an older binary blocked on a Transient Failure (`blocked_reason = 'not_retryable'`) is unblocked by the next driving command with the usual fresh budget ([G. Leaving `blocked`](../../docs/architecture/G-state-machines.md#leaving-blocked)); its next Attempt then follows the new rules. No data repair.

### Agent slots across processes

- **Slots are lock files** `.surt/runtime/slots/agent-0.lock … agent-{N-1}.lock`, locked with the same separate-file `File::lock`/`try_lock` primitive as the run locks ([ADR 0002](../../docs/adr/0002-file-locks-over-sqlite-leases.md)). The directory is created on first use. Lock files are never deleted; `surt delete` does not touch them, since they belong to the project, not to a Subject.
- **Acquisition.** Before each tick, the Driver reads `max_concurrent_agents` from the project configuration and `try_lock`s each slot index below it that it does not already hold. The indices it obtains are the free slots passed to `decide()`, which already assigns one slot per `Launch`. A slot taken for a launch is held until that Execution's flight lands, then released. Slots tried but not used by a launch are released in the same tick, so a Driver never sits on a slot it does not need.
- **Waiting.** With no free slot, ready agents stay queued and the Driver retries on its next tick. **Decided here:** the retry cadence is the Driver's existing tick (about one second) instead of A's "~500 ms"; A is updated. There is no fairness across processes in v1 (A).
- **Live limit.** A lowered limit stops offering slot indices at or above it; agents already holding them finish. A raised limit offers the new indices on the next tick. **Decided here:** if the project configuration fails to parse while a Driver runs, the Driver keeps its last valid limit and emits one warning progress event until the file parses again. Only `max_concurrent_agents` is read live; the frozen keys of D's `config` snapshot stay frozen.
- **`execution.slot`** keeps recording the slot index, for display only. Recovery does not read it: a dead Driver's slots were freed by the OS.
- **Counting for display (decided here).** `status`, the per-Task `waiting for agent slot (N/M in use)` detail and `status --json`'s `agent_slots` count running agent Executions (those holding a `slot`) across **every run of the project whose Driver is live**, from SQLite. `status` never probes the slot lock files, because a probe would briefly hold a slot and could make a Driver skip a launch ("reads never write", J). The footer's `slots N/M` uses the Driver's own `try_lock` results (`M` minus free), which already includes the other Drivers.
- **Unchanged interface.** The CLI still sees only the `Project` facade. The per-Driver semaphore goes away and with it the B rule exception for `Flights` counting slots inside one Driver.

### Global configuration

- **Location:** `~/.surt/config.toml`, `%USERPROFILE%\.surt\config.toml` on Windows, found through the same home-directory lookup Surt already uses for worktrees. With no home directory or no file, the defaults apply.
- **Contents:** one key, `stop_grace_period`, default `"10s"`, parsed with the existing Check `timeout` duration format (positive amount, unit suffix). `deny_unknown_fields`: an unknown key (including any project key) is an error.
- **Loaded by `Project::open`** alongside the project configuration, as B says. A parse error is the existing `InvalidConfig` refusal carrying the global file's path, line and reason, so every command that opens the project exits `3` with J's message shape.
- **Where it applies:** every graceful stop of an agent process tree before a forced kill: the Claude Code runtime's stop (Ctrl-C, closed terminal, `surt stop`, `surt cancel` with a live Driver) and reconciliation stopping surviving agents of a dead Driver. The two hard-coded 10-second constants go away. `surt stop`'s wait keeps having no timeout, now bounded by the configured grace period.
- The global file is not frozen into runs; it is a property of the machine, read by each process at open.

### Documentation updates

- **K:** the three M3 rows of "Mechanisms deferred" say they are implemented; the M3 row of "After Milestone 1" changes status like M2's did.
- **A:** the slot retry cadence and the live-limit warning.
- **I:** the "decided here" points of the pause (latest reset wins, past reset is no reset, step advances per failed resume, no probe), and the status line for a paused run.
- **J:** the paused line in `status` detail, the `paused` field in the `status --json` document, the project-wide `agent slots` count, and the global configuration's refusal behaviour.
- **D:** the `agents.paused` / `agents.resumed` field list.
- **B:** `Ctx` and `Progress` as built; the per-Driver slot exception removed.
- **`CONTEXT.md`:** a new term **Agent Slot** (one of the project's `max_concurrent_agents` places an agent Execution must hold to run; _Avoid_: worker, seat). The **Transient Failure** entry gains that Surt pauses every agent launch of the Driver.
- **Smoke checklist:** an M3 section in the manual Claude Code smoke.

## Testing Decisions

- **A good test checks external behaviour through the highest seam that can observe it**: rows and events through the `Project` facade, exit codes and terminal output through the binary. It never asserts on private Driver state, and no result depends on a fixed sleep: every wait is bounded polling with a generous ceiling (the existing B exception for real processes and OS locks).
- **Main seam: `Project::drive` with the Fake Runtime**, as in the existing engine drive tests (limits, agents, per-item). The Fake's `limit_failure` behaviour already takes a `resets_at`; tests set it a couple of seconds ahead of `now`.
  - A Builder that fails `transient` with a near reset pauses the Driver, writes `agents.paused` then `agents.resumed`, launches the next Attempt after the reset, spends no Attempt, and the run reaches `succeeded`.
  - During the pause no agent Execution starts, while a ready Operation (a Check or an integration of another Task) still runs.
  - An agent already running when the pause starts finishes and is accepted.
  - `not_retryable` still blocks at once; `retryable` still spends an Attempt.
  - A graceful stop during the pause ends with `Interrupted`; driving again starts unpaused and launches at once.
  - **Slots:** two `Project` handles on one project directory drive two Features at once with `max_concurrent_agents = 1` and Fake Executions that wait until stopped; at no point do two agent Executions run. Killing (dropping) one Driver lets the other launch. Lowering the limit in `.surt/config.toml` mid-run is honoured on the next launch, and an invalid file mid-run keeps the last limit.
  - `status` through the facade reports the project-wide slot count and the derived pause.
- **`decide()` table tests** (existing internal seam): with `paused_until` in the future no `Launch` but `RunOperation` still emitted; with it past, launches resume; a `transient` last Execution no longer yields a block. These replace the M1 table rows that expected `transient` to block.
- **Pause schedule table tests** (a pure function taking `now`): reset in the future, reset in the past, no reset with each backoff step up to and beyond 15 minutes, latest of several resets, step reset after a non-transient end, step advancing only on a failed resume.
- **Binary tests** (`surt` crate CLI tests with TOML Fake scripts, as for every command so far): the paused and resumed lines on stderr; `status` and `status --json` showing a paused run from a second process; `agent slots: N/M in use` with two Features driven at once; an invalid `~/.surt/config.toml` (the tests already point `HOME`/`USERPROFILE` at a sandbox) refusing a command with exit `3`; the ASCII fallback `[paused]`. The existing test that expects a rate-limited Task to block with exit `4` changes to expect it to pause and merge.
- **Claude Code stub-process tests** (existing, real processes): a stub that ignores the graceful stop is force-killed after the configured `stop_grace_period`, not after 10 seconds.
- **Config unit tests** (existing pattern for project configuration): defaults, a valid duration, an unknown key, a project key in the global file, a zero or malformed duration.
- **CI** runs fmt, clippy `-D warnings` and `cargo test --features test-support` on macOS, Linux and Windows, as before. Real Claude Code stays manual.
- **Manual macOS smoke** (new M3 section of the checklist): two Features in one toy project driven at once with `max_concurrent_agents = 1` never run two Builders (checked with `surt status` from a third terminal); `stop_grace_period = "30s"` in the global file and a `surt stop` that lets a Builder exit. A real rate limit cannot be provoked on demand, so the pause is covered by the Fake and the stream fixtures only.

## Out of Scope

- Coordinating the pause across Drivers, other Features or other projects on the same machine (map: out of scope for v1).
- Fairness or a shared FIFO for slots across processes (A: none in v1).
- Machine-wide slots shared by several projects.
- Persisting the pause across driving commands (I: the pause lives in memory).
- A ceiling on the pause, a probe launch, or a configurable backoff schedule.
- Any other global configuration key (default model, effort, editor, colours).
- Reclassifying Claude Code errors, or capturing a real rate-limited or network-failure stream: the classification of M1 stands.
- Everything K assigns to M4: `.surt/agents/` overrides, `merge_into_base = "ff-only"` with `--merge`/`--no-merge`, `on_success = "keep"`.
- A daemon, `--detach`, or any supervisor that restarts a Driver after a crash or reboot (map: out of scope).

## Further Notes

- **The classification of a real rate limit is still unverified.** The fixture capture recorded `authentication_failed` and `model_not_found` but no rate-limited or network failure ([I](../../docs/architecture/I-recovery.md#transient-failures-network-and-rate-limits)). The unit tests use synthesized streams that follow the documented `rate_limit_event` shape. If a real one is captured during M3, add it to the fixtures; until then a misclassification costs Attempts (the safe direction) rather than waiting forever.
- **Why M3 after M2** (K): a blocked run already says what to do and when to come back; M3 removes the need to come back.
- **Two Drivers, one rate limit.** Shared slots reduce how often two Features hit the account limit together, but when they do, each Driver pauses on its own failure and resumes on its own schedule. That is acceptable for v1 because a resume costs no Attempt.
- **Shipping** follows Milestones 1 and 2: M3 ships once the suite passes in CI on the three OSes and the M3 section of the manual smoke passes on macOS. Tickets for M3 are cut from this spec into `.scratch/milestone-3/issues/`.
