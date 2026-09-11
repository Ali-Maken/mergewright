# m0-foundations/01 — Workspace scaffold, lint and format config

Status: todo
Build with: Sonnet
Blocked by: —

Why: nothing else can build without the workspace; the lint/parallelism config is a hard rule (CLAUDE.md, PROMPT.md §7.1).

## Read first (keep context lean)
- ARCHITECTURE.md (whole file — it is one page)
- PROMPT.md §7.1 (laptop-safe limits) and the Technology section
- DESIGN.md §8.1 only if the crate table alone is unclear

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
- [ ] `cargo build --workspace` succeeds; 7 packages, edges exactly per ARCHITECTURE.md.
- [ ] A deliberate `x.unwrap()` in `graft` fails clippy; the same in a `#[cfg(test)]` module passes (verify manually, then remove).
- [ ] `.cargo/config.toml` and profiles match PROMPT.md §7.1.
- [ ] ARCHITECTURE.md notes the package-name prefix.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
