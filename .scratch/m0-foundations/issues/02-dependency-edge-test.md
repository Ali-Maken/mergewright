# m0-foundations/02 — Dependency-edge test

Status: done
Build with: Sonnet
Blocked by: 01

Why: ADR-0006 / docs/design/08-architecture.md §8.5 — Cargo rejects cycles but not a stray acyclic edge (`ui → git` compiles fine and breaks the layering). This test makes erosion fail the fast tier.

## Read first
- docs/design/08-architecture.md §8.5 and ADR-0006
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
- [x] Test passes on the scaffold as committed.
- [x] Temporarily adding `graft-git` as a dependency of `graft-ui` makes the test fail with a readable message (verify, then revert).
- [x] Runs in well under 10 s (PROMPT.md §7.1: slow fast-tier tests are bugs). No network.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Notes (build)

- Test: `crates/graft/tests/it/dependency_edges.rs`, wired in from `tests/it/main.rs`.
  `serde_json` added as a dev-dependency of `graft` (not `cargo_metadata`).
- Uses `cargo metadata --format-version 1 --no-deps --manifest-path <graft>/Cargo.toml`.
  `--no-deps` was added beyond the ticket's wording: it skips registry resolution
  entirely, which is what makes "no network" true rather than merely likely, and it
  also means every package in the output is a workspace member (no id->name mapping).
- Dev-deps excluded, normal + build deps counted; the reason is in the file's module doc.
- Verified by hand, not just by green gates:
  - Passes on the scaffold as committed: 1 test, 0.01 s (whole `cargo test`: ~2 s).
  - Added `graft-git.workspace = true` to `graft-ui` -> test FAILS with
    "Not allowed (declared in a Cargo.toml, absent from the table): graft-ui -> graft-git".
    Reverted; green again.
  - Removed `graft-term` from `graft-ui` -> test FAILS with
    "Missing (in the table, absent from every Cargo.toml): graft-ui -> graft-term",
    proving the other half of the diff renders too. Reverted.
