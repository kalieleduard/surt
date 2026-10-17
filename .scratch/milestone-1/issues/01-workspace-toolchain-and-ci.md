# 01: Workspace, toolchain and CI matrix

Spec: [Milestone 1](../spec.md) · Slice: S1 Skeleton · Plan: [B](../../../docs/architecture/B-workspace.md), [K](../../../docs/architecture/K-roadmap.md)

**What to build:** the throwaway `ferris-says` crate is replaced by the two-crate workspace of [ADR 0011](../../../docs/adr/0011-two-crates-keep-the-engine-generic.md): `surt-engine` (lib) and `surt` (bin). The toolchain is pinned, and every PR runs fmt, clippy, tests and a line-coverage gate on macOS, Linux and Windows. The `surt` binary exists with clap and nothing else yet: an unknown command or flag exits `2`.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [x] The workspace has exactly two crates; `surt-engine` cannot depend on `surt`, so the compiler rejects any import of the feature workflow from the engine
- [x] `edition = "2024"`, `rust-version = "1.94"`, stable pinned in `rust-toolchain.toml`, multi-thread Tokio; only dependencies from the table in B are allowed, and none of the forbidden ones (`async-trait`, `git2`/`gix`, `chrono`/`time`, `indicatif`/`crossterm`, `include_dir`, `dirs`)
- [x] The `surt-engine` module split of B exists (`project`, `workflow`, `drive`, `reconcile`, `artifacts`, `context`, `runtime`, `operations`, `git`, `cleanup`, `store`, `platform`), even if most modules are empty
- [x] A `test-support` cargo feature exists, off by default
- [x] CI runs `cargo fmt --all -- --check`, `cargo clippy --all-targets --features test-support -- -D warnings` and `cargo test --features test-support` on macOS, Linux and Windows, with Git ≥ 2.38 pinned per runner
- [x] CI measures project-wide line coverage (e.g. `cargo-llvm-cov`) with `--features test-support` and fails at or below 90%, per `rules/testing.md`; no production code is excluded to reach the threshold
- [x] `surt` with an unknown command or flag exits `2` on all three OSes, verified through the binary
