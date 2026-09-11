# m0-foundations/02 — Dependency-edge test

Status: todo
Build with: Sonnet
Blocked by: 01

Why: ADR-0006 / DESIGN.md §8.5 — Cargo rejects cycles but not a stray acyclic edge (`ui → git` compiles fine and breaks the layering). This test makes erosion fail the fast tier.

## Read first
- DESIGN.md §8.5 and ADR-0006
- ARCHITECTURE.md crate table (the edge set to assert)

## Goal
A fast-tier test in `graft`'s `tests/it/` that parses `cargo metadata` and asserts the internal dependency edge set matches ARCHITECTURE.md exactly — no extra edges, no missing ones.

## Scope
In:
- Run `cargo metadata --format-version 1` from the test, parse with `serde_json` (dev-dependency; don't pull the heavier `cargo_metadata` crate), extract workspace-member → workspace-member edges (normal + build deps; dev-deps excluded so test-only helpers don't trip it — document that choice in the test).
- Assert equality with a hard-coded expected set, with a comment: "adding a crate or an allowed edge means updating this test in the same commit (ADR-0006)".
- On failure, print the diff of edges both ways.

Out:
- Enforcing anything about external crates.
- CI wiring.

## Acceptance
- [ ] Test passes on the scaffold as committed.
- [ ] Temporarily adding `graft-git` as a dependency of `graft-ui` makes the test fail with a readable message (verify, then revert).
- [ ] Runs in well under 10 s (PROMPT.md §7.1: slow fast-tier tests are bugs). No network.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
