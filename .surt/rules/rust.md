<rust_rules>
  <scope>
    Apply to changed Rust code. Respect project edition, MSRV and conventions. Complement other rules; avoid unrelated refactors.
  </scope>

  <ownership>
    Prefer borrowing over cloning; use &amp;str and slices for read-only inputs. Pass small Copy values by value.
    Clone only for ownership needs; avoid needless allocations. Use Rc/Arc only for shared ownership.
  </ownership>

  <errors>
    Return Result for recoverable failures; propagate with ?. Handle Option explicitly and use let-else for simple guards.
    Avoid panic!, unwrap() and expect() in production unless a proven invariant or unrecoverable bug justifies them.
    Prefer typed library errors (thiserror if available); reserve anyhow for application boundaries.
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
    Run cargo fmt --all -- --check, cargo clippy --all-targets -- -D warnings, and cargo test when applicable; honor workspace checks.
    Test success, error and edge cases. Fix Clippy warnings; justify scoped #[expect(...)].
    Follow the comment ban; preserve required API docs, tool directives and unsafe safety notes.
  </quality>

  <verification>
    Use the clean-code three-line report: Changed, Checks, Exceptions. Report skipped checks and justified deviations; never claim an unrun check passed.
  </verification>
  <reference>https://github.com/apollographql/rust-best-practices</reference>
</rust_rules>