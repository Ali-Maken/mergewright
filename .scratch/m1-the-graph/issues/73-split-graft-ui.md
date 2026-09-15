# m1-the-graph/73 — Review M1: split graft-ui under the 400-line rule

Status: ready
Build with: **Sonnet** (medium) — mechanical moves along existing seams; no behaviour change.
Blocked by: 65, 66, 67, 68, 69, 72

From the M1 review round 1, "M1" (PROMPT.md 6.2: files over 400 non-test lines). Split into 70–76, one crate area each, after the fix tickets so moves don't conflict with fixes.

## Read first
- docs/reviews/m1-the-graph.md "M1" only; PROMPT.md §6.1–6.2; ARCHITECTURE.md crate map section.
- The `codebase-design` skill (module changes).
- The crate MAP.md files for: crates/graft-ui/src/graph_screen/mod.rs (~1,115), graph_column/mod.rs, terminal.rs.

## Scope
In: split crates/graft-ui/src/graph_screen/mod.rs (~1,115), graph_column/mod.rs, terminal.rs, graph_screen by pane. Each resulting file ≤400 non-test lines, one concern per file. Pure moves plus the visibility changes they need; public API unchanged unless a MAP.md line must move. Update MAP.md, and ARCHITECTURE.md if structure changes. Before finishing, list every file in these crates over 400 non-test lines (`wc`-based, tests excluded) and fix the ones this ticket names; list the rest in the report.
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
