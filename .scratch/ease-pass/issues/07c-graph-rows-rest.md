# 07c — graph rows, the rest of 07

Status: ready (split by conductor 2026-10-18: 07 compacted at 8/14, f2269602 merged)
Blocked by: 07 part (merged)
Build with: Opus (`builder-opus`) — same model as 07; scope, signatures and boxes unchanged.

## Scope
The open boxes of `07-graph-rows.md` and its NOTES "Remaining (next builder)" list, in that order: branch / who / age columns (SHA + author chip dropped), `↑N to push` and `not pushed yet` labels on every local branch (07a's `RefLabel::ahead_of_upstream()`), `ref_spans` and drag, preview, the four `main-*` captures. Tick the boxes **in 07's file**.

Rulings (conductor, from 07 NOTES questions):
- `you` on own rows: the mock draws it, so it is in scope. Read `user.email` once with the repo config read (no per-row read); a commit whose author email matches reads `you`. Test on a fixture with two authors.
- `not merged`: no source until 09 wires `branch_status::Merged`. Not built here; stays a question in 07 NOTES for 09.
