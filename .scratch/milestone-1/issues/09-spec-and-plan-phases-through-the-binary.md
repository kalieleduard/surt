# 09: Spec and plan phases through the binary

Spec: [Milestone 1](../spec.md) (stories 16–20, 24–28, 35–37, 40, 94, 104) · Slice: S3 Engine + Fake · Plan: [J](../../../docs/architecture/J-cli.md)

**What to build:** the developer drives the first two phases from the terminal with the Fake: `surt feature` produces the spec and stops at its gate, `approve`/`reject` record decisions and print the next command, `surt feature <full-name>` resumes the spec phase, and `surt plan` produces the manifest and `tasks/*.md` and stops at its gate. Slice S3 is done when spec → gate → reject → spec rev 2 → approve → `plan` → gate passes through the binary.

**Blocked by:** 06 (Built-in Catalog and `surt feature` creates a Feature), 08 (`drive` through agent steps and Approval Gates)

**Status:** ready-for-agent

- [ ] With `test-support`, `SURT_RUNTIME` selects the Fake with the path to a TOML script; release builds cannot select it
- [ ] `surt feature "<description>"` drives the Spec Agent and exits `0` at the spec gate; `surt feature <full-name>` resumes (regenerating after a rejection) and, on an approved spec, points at `surt plan` without changing anything
- [ ] `surt approve` prints what it approved, its SHA-256 and author, and the next command from the gate → next-command table; `surt reject -m` prints the command that regenerates; neither drives
- [ ] `surt plan` refuses unless the spec is approved, then drives the Task Agent to the manifest gate; the manifest and one Markdown file per Task appear in the Feature's folder
- [ ] A `<feature>` argument accepts the full name, a unique slug or a unique prefix; an ambiguous one refuses listing candidates
- [ ] Driving commands refuse with exit `3` when `.surt/agents/` exists
- [ ] Binary test of the full S3 "done" flow, plus refusals (plan before approval, ambiguous name, `--rev` mismatch)
