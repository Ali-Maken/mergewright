# m1-the-graph/76b — Review M1: split graft-term kitty.rs under the 400-line rule

Status: done
Build with: **Sonnet** (medium) — mechanical moves along existing seams; no behaviour change.
Blocked by: 76a

From the M1 review round 1, "M1" (PROMPT.md 6.2: files over 400 non-test lines). Split from 76: 70 and 71 each compacted on one or two big files, so one big file (or two small ones) per ticket. After the fix tickets so moves don't conflict with fixes.

## Read first
- docs/reviews/m1-the-graph.md "M1" only; PROMPT.md §6.1–6.2; ARCHITECTURE.md crate map section.
- The `codebase-design` skill (module changes).
- The crate MAP.md files for: crates/graft-term/src/kitty.rs (448).

## Scope
In: split crates/graft-term/src/kitty.rs (448), along existing concerns. Each resulting file ≤400 non-test lines, one concern per file. Pure moves plus the visibility changes they need; public API unchanged unless a MAP.md line must move. Update MAP.md, and ARCHITECTURE.md if structure changes. Before finishing, check the named files are ≤400 non-test lines (`wc`-based, tests excluded). Don't survey other files: the conductor holds the full list. Use `wc`/`grep` and a scout to find cut points; move code with shell tools rather than reading whole files into context.
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
