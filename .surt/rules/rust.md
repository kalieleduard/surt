<rust_rules>
  <scope>
    Apply to changed Rust code. Respect the workspace edition (2024), MSRV (rust-version in Cargo.toml) and the pinned toolchain in rust-toolchain.toml. Complement other rules; avoid unrelated refactors.
  </scope>

  <ownership>
    Prefer borrowing over cloning; use &amp;str and slices for read-only inputs. Pass small Copy values by value.
    Clone only for ownership needs; avoid needless allocations. Use Rc/Arc only for shared ownership.
  </ownership>

  <errors>
    Return Result for recoverable failures; propagate with ?. Handle Option explicitly and use let-else for simple guards.
    Avoid panic!, unwrap() and expect() in production unless a proven invariant or unrecoverable bug justifies them; tests may use them.
    Use typed errors with thiserror; carry the context a user needs to act (path, command, step) in the error, not in logs alone.
  </errors>

  <design>
    Use enums to model valid states; introduce typestate only when it prevents meaningful misuse.
    Prefer generics or impl Trait; use dyn Trait for runtime polymorphism or type erasure.
    Keep traits focused. Avoid needless Box, Mutex, global state and unsafe code.
  </design>

  <performance>
    Avoid redundant clone(), collect(), heap allocations and I/O inside loops. Prefer lazy iterators when readable.
    Evaluate time/space complexity; benchmark optimized release builds before sacrificing clarity.
  </performance>

  <quality>
    <why>CI runs these exact commands on Linux, macOS and Windows with RUSTFLAGS=-D warnings; running anything weaker locally hides failures until CI.</why>
    Run, from the workspace root:
    - cargo fmt --all -- --check
    - cargo clippy --all-targets --features test-support -- -D warnings
    - cargo test --features test-support
    Test success, error and edge cases. Fix Clippy warnings; justify any scoped #[expect(...)] in Exceptions.
    Keep code portable across the three CI platforms: build paths with Path/PathBuf, never assume "/" separators or case-sensitive file systems.
    Follow the comment ban in clean-code.md; preserve required API docs, tool directives and unsafe safety notes.
  </quality>

  <verification>
    Report with the three-line format of clean-code.md, including skipped checks and justified deviations.
  </verification>
  <reference>https://github.com/apollographql/rust-best-practices</reference>
</rust_rules>
