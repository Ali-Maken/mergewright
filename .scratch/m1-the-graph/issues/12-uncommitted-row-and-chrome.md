# m1-the-graph/12 — The uncommitted-changes row and 16-colour chrome

Status: ready
Build with: **Sonnet** (medium) — a read, a virtual row, a separator rule
Blocked by: 10
Leaves runnable: yes — a dirty working tree shows as a row above HEAD.

Why: "what have I not committed" is a daily question, and at 16 colours the panes currently melt into one background.

## Read first
- PROMPT.md §1.3 (uncommitted changes paragraph only)
- DESIGN.md §1.1 (the mockup's top row) and §4.6
- KNOWN_GAPS.md M0 row "Dark chrome flattens at 16 colours"
- Code: `crates/graft-ui/src/theme/` (depth mapping), any existing status read in `crates/graft-git/src/` (grep `status`), 10's row widget

## Scope
In:
- A virtual row above HEAD's row, on HEAD's lane, with staged / unstaged / untracked counts. Absent when the tree is clean.
- Counts read off the UI thread, refreshed whenever the app re-reads history or refs. No file watcher.
- At `ColorDepth::Palette16`, pane separation comes from a rule or box-drawing edge, not a background step. Closes the M0 gap.

Out:
- Staging anything (M3). Lane identity at 16 colours (13).

## Acceptance
- [ ] Fixture tests: clean (no row), staged only, unstaged only, untracked only, all three.
- [ ] Unborn branch with staged files: decide where the row attaches, snapshot it.
- [ ] Snapshot at 16 colours shows the panes visibly separated.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
