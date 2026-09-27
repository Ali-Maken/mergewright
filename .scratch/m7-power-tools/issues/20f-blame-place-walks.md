# 20f blame-place-walks — 20's walks and captures

Status: partial — all but the shallow capture; that one is blocked: Graft cannot open a shallow clone (NOTES)
Blocked by: 20g (same files), 20h (the pane bar loses `space fold` on a file row; the re-shoots and hint sweeps must see it — planner, 2026-09-27).
Crates: graft (tests, `docs/shots/`).
Build with: Opus (unchanged from 20).
Scout: none. Read 20's NOTES and 20e's report.
Split by: conductor, 2026-09-26, along 20's builder's seam (20-b). No new decision.

## Scope
From `20-blame-place.md`: the m7_walk journeys (plus the pinned-base walk from the ruling), the 7 captures + `blame-then-commit-with-base`, the pane-bar re-shoots, the hint sweep tests, and the shallow capture. Tick 20's **Walks** and **Captures** boxes in place, each naming its test. Captures ≤ 200 KB, each with its "Right:" line. They are judged by eye at the M7 stop.

The sweep form is 20h's, not this ticket's: the hint sweeps here run over every `PaneFocused` form, none left out.

## NOTES (builder, 2026-09-27)

Not compacted. No scout: 20e/20g/20h NOTES named every seam, and this ticket touches tests and captures only.
Fixture: `Shape::Session`, copied, plus `git commit-graph write --reachable` so the commit-graph offer never shows
(it would take `B`); every walk and capture asserts `commit_graph_line() == None`.

**Walks** (`crates/graft/tests/it/m7_walk/blame.rs`, counted by `m7_walk::noted_keys`, which reads every
`journey-counts` fence; the duplicate-id test already exists as
`m5_walk::journeys::tests::a_duplicate_id_across_fences_is_refused`). Setup (not counted): `⏎` on `main`, `j` to
`src/auth/session.rs`, `⏎`, `]` (one more context line, so line 22 shows), `j` to new line 22.
- `blame_line_to_commit` `B ⏎` = 2: lands on Lina's `2f48ca3`, `src/session.rs` (her path, before the rename), cursor
  New **21** (her numbering of main's 22), graph cursor on her row, trail hidden, focus Diff.
- `commit_back_to_blame` `Esc` = 1, and `commit_back_to_blame_with_a_base_pinned` (base `0a27627` pinned in setup):
  blame back on line 22; with the base, nothing pinned and `origin.base` still set aside.
- `blame_who_at_line` `j` = 1: setup `B k` (Tom's 21), `j` → 22; exactly one `◂` on screen, on the `▸` row, naming Lina.
- `blame_line_before` `B ^` = 2: two places; cursor on parent line 20, Tom, `if resp.status() == 401 {`; title
  `before 2f48ca3`.
- Red: all green on first run (the code existed). Count check shown red by mutation (an extra route entry in
  `blame_line_before` → "took 3 inputs … the note allows 2"), reverted.

**Captures** (`crates/graft/tests/it/blame_place.rs`, `check_shot`, drawn by `view::view` via `frame_at`, not a pty —
the same mechanism as 20b's and 20h's m7 captures; ages against `CORPUS_NOW`). Each test's doc comment carries its
Right line. Sizes 1.7–4.3 KB.
- `blame-187x50.txt`, `blame-140x40.txt`, `blame-100x30.txt`, `blame-long-line-187x50.txt`, `blame-find-187x50.txt`,
  `blame-then-commit-120x40.txt`, `blame-then-commit-with-base-120x40.txt`.
- **Differences from the spec's words, each a fixture fact, not a code change:**
  - The mock-ups' `a1b2c3d` is the fixture's `2f48ca3`.
  - 140x40: the trailer sheds ` · 2f48ca3` — the fixture's line 22 is long enough that the whole trailer would pass
    140 columns. The spec said "the trailer whole". Asserted as drawn.
  - find: the spec's `/retry⏎ n` has one match in the fixture (line 27), so "2 of N" can't be shown. Used
    `/refresh⏎ n`: `⏎` → line 27 (7th), `n` → line 47, title `… · find "refresh" 8 of 8`.
  - then-commit: the diff's `▎` is on Lina's line 21 (the spec's "line 22" is the final number; `⏎` lands on
    `orig_line` by 20's own rule, `20-blame-place.md:63`).
  - with-base: the status replaces the bar row (the set-aside words are a `Status::Waiting`, 20e's deviation), so that
    frame has no `esc back to blame`; the other landing does. `▏` appears once in both, on the graph's cursor row
    (Lina's) — not a base mark. Asserted: no `▏` on Priya's row, no `x base`, no `COMPARE`.
- Question (rule 7, no line checked): at 187, a cursor row in the middle of a block draws only the name in the gutter
  (`▸         Ana` in `blame-find-187x50.txt`), no sha. By eye for the user; 20c's drawing, not changed here.
- Colour unverified (rule 5): heatmap, cursor tint, dimming.

**Shallow capture — blocked, not built.** `blame-shallow-187x50.txt` needs Graft to open a `--depth 2 file://` clone,
and it can't:
- The history walk follows the boundary commit's parent: `reading history: commit b32274b… is not in the repository`
  (`graft-git/src/history/node.rs:76`, `NodeError::Missing`; nothing in `graft-git/src/history/` reads
  `<common_dir>/shallow` — grep `shallow` finds only a doc comment at `read.rs:111`). Seen with `GRAFT_TEST_TRACE=1`.
- The radar's context job fails the same way, and `update/radar.rs:331` re-requests on `Loading::Failed(_)` at the end
  of every `update`: each failure is an update, so it re-asks forever. The test spun at 100 % CPU for 9 min
  until killed (`App::pump` never timed out because events kept coming). That is an unbounded retry loop (rule 2).
- Both are outside this ticket's crates (graft-git; graft source). Not worked around. 20g's shallow Part C tests use
  a built state, so they did not meet it. The capture test was removed before commit, not ignored.
- Needs: a ticket for shallow-clone history (read the shallow list, treat those commits as parentless) and one for
  the radar's retry-on-failure. Then the capture: clone, `commit-graph write` (it may refuse on a shallow repo — the
  offer showed `Absent` in the trace; check the walk's no-offer assert), `B` on 22, `k`×4 to 18.

**Re-shoots:** none made. The five the scout named (`m6/fix-three-panes-{180,187,200}x50.txt`,
`fix-c-in-pane-finishes-187x50.txt`, `stash-clash-take-esc-pane-120x40.txt`) are pinned byte-for-byte by
`resolver/three_panes.rs:107`, `resolver/shell_begun.rs:1278`, `m6_stash_put_back/pop.rs:129`, green in this run, so
they already draw what the code draws. `B blame` appears in no pre-existing capture (grep `docs/shots`).
**Not verified:** that every capture with a pane bar is pinned. A basename grep of `crates/` finds ~40 bar-drawing
files under `docs/shots/` with no literal name (some are named by `format!`, e.g. the m2 ones 20h regenerated; others
may have no writer). A stale unpinned one can't be re-shot without a writer; not chased.

**Hint sweeps:** unchanged here; verified by reading. `catalogue::pane_focused_form` (graft-ui `catalogue.rs:359-381`)
yields all five `(opens, blamable, folds)` forms, no exclusion; `update/tests.rs` sweeps cover `Blame` and
`BackToBlame` (`:3994`, `:4077`, `:4390`, `:4483`). `every_key_the_bar_draws_is_answered` green in this run.

**Left:** the shallow capture (blocked above); the keymap-doc check against 20e's key table (20g's list; not in this
ticket's scope).
