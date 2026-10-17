# 15: `surt start` delivers a Feature

Spec: [Milestone 1](../spec.md) (stories 41, 58, 65, 67) · Slice: S4 Git · Plan: [J](../../../docs/architecture/J-cli.md), [Test strategy](../../surt-architecture/issues/16-test-strategy.md)

**What to build:** the developer runs `surt start <feature>` after approving the manifest and gets back a delivered Integration Branch, with progress lines on stderr. Interruptions and blocks end with the right exit code and a message that says what to type next. Slice S4 is done when the brief's E2E is green through the binary (without `kill` and without cleanup assertions, which arrive in 18).

**Blocked by:** 09 (Spec and plan phases through the binary), 14 (Integration, conflicts and the Shipper)

**Status:** ready-for-agent

- [ ] `surt start` refuses unless the manifest is approved, then drives builders, checks, review, integration and the Shipper
- [ ] Progress events render as J's event lines (`✔ task A merged (…)`, `↻ task B build attempt 2`, `✔ shipper delivered — …`, `✔ run succeeded`)
- [ ] Success exits `0` and prints the delivered branch and SHA; nothing is pushed and the main branch is untouched
- [ ] A stop on a blocked step exits `4`, listing each blocked step with its last error, its log path and the resume command; a transient block shows the reset time when known
- [ ] Ctrl-C, SIGHUP or closing the console triggers a graceful stop and exit `130`; a second signal forces the stop
- [ ] E2E through the binary with the Fake: `init` → `feature` → reject → spec rev 2 → approve → `plan` → approve → `start` with 2+ Tasks, one with a rejected review and a new Builder → integration → Shipper; final state verified by reading Git and SQLite directly; under 60 s per OS with no real sleeps
- [ ] Binary tests for exit `4` and exit `130`
