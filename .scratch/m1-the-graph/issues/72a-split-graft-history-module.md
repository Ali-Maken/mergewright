# m1-the-graph/72a — Review M1: split graft history module under the 400-line rule

Status: done
Build with: **Sonnet** (medium) — mechanical moves along existing seams; no behaviour change.
Blocked by: 71

From the M1 review round 1, "M1" (PROMPT.md 6.2: files over 400 non-test lines). Split from 72: 70 and 71 each compacted on one or two big files, so one big file (or two small ones) per ticket. After the fix tickets so moves don't conflict with fixes.

## Read first
- docs/reviews/m1-the-graph.md "M1" only; PROMPT.md §6.1–6.2; ARCHITECTURE.md crate map section.
- The `codebase-design` skill (module changes).
- The crate MAP.md files for: crates/graft/src/history/mod.rs (713), crates/graft/src/history/scroll.rs (495).

## Scope
In: split crates/graft/src/history/mod.rs (713), crates/graft/src/history/scroll.rs (495), along existing concerns. Each resulting file ≤400 non-test lines, one concern per file. Pure moves plus the visibility changes they need; public API unchanged unless a MAP.md line must move. Update MAP.md, and ARCHITECTURE.md if structure changes. Before finishing, check the named files are ≤400 non-test lines (`wc`-based, tests excluded). Don't survey other files: the conductor holds the full list. Use `wc`/`grep` and a scout to find cut points; move code with shell tools rather than reading whole files into context.
Out: behaviour changes, renames of public items, anything outside the named files' crates.

## Acceptance
- [ ] Named files split; each new file ≤400 non-test lines.
- [ ] No test changed except import paths; all gates green; no snapshot changes.
- [ ] MAP.md (and ARCHITECTURE.md if needed) updated.
- [ ] Report: before/after line counts; any file still over 400 in these crates.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.

## Report
- mod.rs (713) -> mod.rs (40 facade) + state.rs (277) + stream.rs (123) + views.rs (324).
- scroll.rs (495) -> scroll/mod.rs (30 facade, Motion) + scroll/apply.rs (234) + scroll/query.rs (251).
- All new/changed files ≤400 non-test lines. No other file in crates/graft over 400 was touched.
- Public API unchanged (same items re-exported from history::mod); MAP.md needed no change (documents by item, not file). ARCHITECTURE.md has no per-file references to update.
- tests.rs import list extended (Arc, Oid, Follow, Inclusion, Delivery, RepoStore) since `use super::*` no longer pulls them transitively through mod.rs; no test logic changed.
- Gates: fmt --check, clippy --all-targets -D warnings, clippy --all-targets --features heavy-tests -D warnings, cargo test (317 tests) all green.
