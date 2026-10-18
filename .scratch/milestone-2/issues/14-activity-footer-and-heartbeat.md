# 14: Activity snapshot, TTY footer and the 5-minute line

Spec: [Milestone 2](../spec.md) (stories 71–74, 83) · Slice: R8 Terminal · Plan: [J. Foreground progress](../../../docs/architecture/J-cli.md#foreground-progress), [B. Progress and stopping](../../../docs/architecture/B-workspace.md#progress-and-stopping)

**What to build:** a developer can see at a glance that a live run is alive. On a TTY, driving commands show a footer under the event lines, redrawn in place, with what is running and for how long, what is queued, the Driver's slots and the total elapsed time. Without a TTY, a `… still running: B build (15m), C review (3m)` line prints after every 5 minutes without an event, so CI doesn't kill the job.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The engine adds a typed activity snapshot to `Progress`, emitted when it changes: running steps with their start times, the queued count, slots in use and the limit
- [ ] The footer and the 5-minute line are pure functions of (last snapshot, time of the last event line, `now`, terminal capabilities) in the `surt` crate's progress module; a Tokio interval only calls them
- [ ] Footer only when stderr is a TTY, drawn with `\r` plus ANSI erase-line (no `indicatif`/`crossterm`), erased before each event line and before the final summary
- [ ] 5-minute line only without a TTY; no footer, colours or spinner without a TTY
- [ ] Pure render tests: footer text for a given snapshot and `now`; when the 5-minute line fires, with no real waiting
- [ ] Binary test: a non-TTY run's stderr has only event lines
