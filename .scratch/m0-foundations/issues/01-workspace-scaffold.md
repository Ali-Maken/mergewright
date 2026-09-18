# m0-foundations/01 — Workspace scaffold, lint and format config

Status: done
Build with: Sonnet
Blocked by: —

Why: nothing else can build without the workspace; the lint/parallelism config is a hard rule (CLAUDE.md, PROMPT.md §7.1).

## Read first (keep context lean)
- ARCHITECTURE.md (whole file — it is one page)
- PROMPT.md §7.1 (laptop-safe limits) and the Technology section
- docs/design/08-architecture.md §8.1 only if the crate table alone is unclear

## Goal
A Cargo workspace matching ARCHITECTURE.md's crate map, with the lint, format and parallelism config every later ticket relies on. No behaviour yet.

## Scope
In:
- Workspace with 7 packages: `graft-core`, `graft-graph`, `graft-git`, `graft-ops`, `graft-term`, `graft-ui`, and `graft` (the binary, ARCHITECTURE's `app`). Add one line to ARCHITECTURE.md noting the `graft-` package-name prefix (a bare package named `core` collides with Rust's built-in `core`).
- Declare exactly the allowed internal dependency edges from ARCHITECTURE.md's table in the Cargo.tomls, even though the crates are near-empty (ticket 02 locks this edge set with a test).
- Workspace lints: deny `clippy::unwrap_used` and `clippy::expect_used`; `clippy.toml` with `allow-unwrap-in-tests = true` and `allow-expect-in-tests = true` (PROMPT.md §6.3).
- `.cargo/config.toml`: `jobs = 4`, `[env] RUST_TEST_THREADS = "4"` (PROMPT.md §7.1).
- Profiles: `debug = "line-tables-only"` for dev and test (PROMPT.md §7.1).
- Single integration-test binary skeleton for `graft`: `tests/it/main.rs` (PROMPT.md §7.1 — one binary, modules included from it).
- Each crate: `lib.rs` (or `main.rs`) with a one-paragraph crate doc comment saying what it owns, taken from ARCHITECTURE.md. No placeholder types.
- Rust edition 2021 or later, stable toolchain.

Out:
- Any behaviour, any external dependency beyond what compiling needs (ratatui/crossterm etc. come with the tickets that use them).
- The `heavy-tests` feature (first heavy test is M1).
- CI config (none exists; gates run locally).

## Acceptance
- [x] `cargo build --workspace` succeeds; 7 packages, edges exactly per ARCHITECTURE.md.
- [x] A deliberate `x.unwrap()` in `graft` fails clippy; the same in a `#[cfg(test)]` module passes (verify manually, then remove).
- [x] `.cargo/config.toml` and profiles match PROMPT.md §7.1.
- [x] ARCHITECTURE.md notes the package-name prefix.

## Notes (build)

- Layout: `crates/<package>/`, package names `graft-*`, binary package `graft`.
  `prototypes/*` is excluded from the workspace so `graph-spike` stays standalone.
- Edition 2024, `resolver = "3"`, rust-version 1.85 (toolchain here is 1.98.1).
- Lints live in `[workspace.lints.clippy]` (`unwrap_used`/`expect_used` = deny) with
  `[lints] workspace = true` in all 7 manifests; `clippy.toml` re-allows both in tests.
- Profiles (`debug = "line-tables-only"` for dev and test) moved from `.cargo/config.toml`
  into the workspace `Cargo.toml` — one source of truth, and they apply however cargo is
  invoked. `.cargo/config.toml` keeps `jobs = 4` and `[env] RUST_TEST_THREADS = "4"`.
- Verified by hand, not just by green gates:
  - `unwrap()` outside tests → clippy error `used unwrap() on an Option value` (unwrap_used).
  - `unwrap()` + `expect()` inside `#[cfg(test)]` → clippy clean. Both probes removed.
    (Note: use a non-literal `Option` when probing; `Some(1).unwrap()` trips the unrelated
    `unnecessary_literal_unwrap` lint and muddies the result.)
  - `cargo metadata` edge set printed and compared to ARCHITECTURE.md: exact match.
  - A temporary test printed `RUST_TEST_THREADS=Ok("4")` inside a test process, proving the
    `[env]` block reaches test binaries. Removed afterwards.
  - `cargo test` runs exactly one integration binary, `tests/it/main.rs` (target `it`).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
