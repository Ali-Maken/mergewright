# 30d relaunch-notice-ladder — the relaunch notice keeps the stash ref and `J` at 80 columns

Status: done
Build with: **Sonnet**. One pure fitting function with every word and width decided below, one call site, tests with
their exact strings given. No state, no journal, no git.
Needs: none. 30c's `operate::edits_clause` and `AppState::interrupted_edits` are on main (a69322cd).
Blocked by: **34** (graft is taken serially; 30d shares no file with 33, 20k, 20l or 34 by their Crates lines). Queue:
33 → 20k → 20l → 34 → **30d** → 03b → 20j. Before 03b: 03b's pull sets aside too, so a killed pull relaunches into this
notice with a longer description (`pull …`); landing the ladder first means 03b's captures need no second pass.
Crates: graft only (`update/undo.rs` `interrupted_notice` and the new fns, `view/frame/bars.rs` the footer call,
`update/undo/tests/set_aside.rs`, `tests/it/m6_stash_put_back/set_aside.rs` K4, MAP.md). Docs: TESTING.md,
`docs/shots/m7/30c-relaunch-edits-back-*`. Needs another crate → commit at the seam, report `blocked: needs <crate>`, stop.
Scout: none. 30c's NOTES "80 columns" names the sites (`view/frame/bars.rs:307-308` → `sanitize_line`, cut at the
width). Re-read both, and `interrupted_notice`, before editing.
Split by: planner, 2026-09-27, from 30c's open K4 box (NOTES "80 columns").

## Read first
- 30c's ticket: NOTES "80 columns" and "A2" only (`.scratch/m7-power-tools/issues/30c-put-back-review-fixes.md:314-327`).
- 30b's ticket: Decided 3 (the done-line rungs, `30b-put-back-words.md:38-55`) and NOTES "Rung pick" (:256-261).
- graft MAP.md: `interrupted_notice` (:323), `edits_clause` (:260), `edits_status` (:259).
- graft-ui MAP.md: `fitting_line` (:36), `GraphView::problem` (:127).
- `docs/research/ux-principles.md` §2 rows 1, 4, 6, 8, 9.

## Problem → user → solution
- **Problem:** after a kill, at 80 columns the one line that says where his edits are is cut after `a copy stay…`
  (`docs/shots/m7/30c-relaunch-edits-back-80x24.txt:42`). `stash@{0}` and `J` are gone: the line says a copy exists
  but not where, or how to look.
- **User:** him, relaunching Graft after it was killed mid-switch, in a narrow split (80 columns).
- **Solution:** 0 keys to see where the edits are, at every width from 80 up (today: 0 at 120, not answered at 80).

## Decided (planner, 2026-09-27)
1. **Parts.** The notice is four parts, in this order, never reordered:
   `lead` `an operation was interrupted: ` · `description` (the entry's, e.g. `checkout fix`) · `clause` (optional:
   `edits_clause(at)`, or the older `edits in stash@{n}`) · `key` `J to inspect`.
   - **Never cut:** `clause` and `key`. The clause is where the edits are, the key the way on (heuristic 9: what,
     and the next key; heuristic 6: the key visible where it is used).
   - **Clause words are never shortened.** They are the status line's own (`edits_clause`); 30c Decided 3 made the
     two the same on purpose (heuristic 4). The ladder shortens only Graft's framing and his description.
2. **The ladder**, longest first; the first rung that fits the row's text width wins. Each rung is the one above with
   parts removed, so the eye finds every part where it was (the 30b pattern):
   1. `an operation was interrupted: <description> · <clause> · J to inspect`
   2. `interrupted: <description> · <clause> · J to inspect` — `an operation was` goes first: it says nothing
      `interrupted` and `J` don't (heuristic 8).
   3. `interrupted: <description cut>… · <clause> · J to inspect` — the description cut from the right to fill the
      width, trailing spaces trimmed before `…`, **only if at least 10 cells of it remain, `…` included**. The verb
      and the start of the branch survive (`rebase feature/payments-retry…`).
   4. `interrupted · <clause> · J to inspect` — the description dropped. The graph shows `HEAD` where the operation
      left it, and `J` names the entry.
   - Narrower than rung 4 (below 78 columns for the longest clause, `S p` with n < 10): rung 4, cut at the end with
     `…` by the footer's existing cut. Not laid out further; say so in NOTES.
   - No clause: the same rungs without ` · <clause>`.
3. **Width, measured where it is drawn.** Display cells (not bytes, not chars) against the footer row's text width
   (`columns − 1`: the row indents one cell, 30b NOTES). Use the measure the footer's cut already uses; don't add a
   second one. **The rung is picked at draw time**, from the frame's width, so a resize re-picks. (30b picks at answer
   time and a resize leaves a stale rung, a known gap; the notice has no such reason: it is derived every frame.)
4. **Sanitize before measuring.** `description` is his text (a commit subject can be one). It goes through
   `sanitize_line` before the fit, so the cells counted are the cells drawn. The footer's final cut stays as a guard.
5. **Several entries.** MAP :323 says "entries" but not how two are worded. Read it. Whatever today's line puts
   between the lead and the first ` · ` is `description`; if several are joined, the join is `description` and rung
   3 cuts it. If a plural lead exists, rung 1 keeps it and rungs 2–4 use `interrupted` for it too. Record the words
   in NOTES.

### Widths, worked (the tests assert these strings; `D` = `checkout fix`, 12 cells)
| Clause | Rung 1 | 2 | 4 | At 80 columns (79 cells) |
|---|---|---|---|---|
| `your edits are back · a copy stays in stash@{0}` | 107 | 90 | 76 | rung 4: `interrupted · your edits are back · a copy stays in stash@{0} · J to inspect` |
| `your edits are in stash@{0} · S p puts them back` | 108 | 91 | 77 | rung 4: `interrupted · your edits are in stash@{0} · S p puts them back · J to inspect` |
| `edits in stash@{0}` (a later launch) | 78 | — | — | rung 1, unchanged from today: `an operation was interrupted: checkout fix · edits in stash@{0} · J to inspect` |
| same, `D` = `rebase feature/payments-retry onto origin/main` (46) | 112 | 95 | 47 | rung 3: `interrupted: rebase feature/payments-retry… · edits in stash@{0} · J to inspect` (79) |
| none, `D` = `checkout fix` | 57 | — | — | rung 1 |

Builder: recount each with the measure you use before asserting. A disagreement with this table means the table is
wrong: say so in NOTES and assert what the rule gives.

## Journeys
| Journey | From | Keys | Count | Best tool | Lands |
|---|---|---|---|---|---|
| `relaunch-after-kill` at 80 (changed) | 30c's: killed after the put-back, before the drop; relaunched in 80x24 | none | 0 to see where the edits are | shell: `git status` + `git stash list`, then judge whether the stash is a duplicate = 2 commands and a judgement | graph; footer `interrupted · your edits are back · a copy stays in stash@{0} · J to inspect`, whole |
| …then `u` | the footer above | `u` | 1 | — | as 30c: before the switch, staging included, stash list as before |

## Scope
**In:** Decided 1–5; the tests below; the 80x24 capture regenerated; the docs under Touches.
**Out:** any change to `edits_clause`'s words or to the status line's fit; the J browser's row; 30b's answer-time
rung pick (its own gap); A2 (below, for the conductor).

## Signatures
```rust
// crates/graft/src/update/undo.rs (beside interrupted_notice)
/// The relaunch notice fitted to `cells` display cells: the first rung of 30d's ladder that fits,
/// else the last (the footer's cut then ends it with `…`). `clause` is never cut or reworded.
pub(crate) fn notice_line(description: &str, clause: Option<&str>, cells: usize) -> String;

/// `interrupted_notice` fitted to the footer row. The footer's only caller (`view/frame/bars.rs`).
pub(crate) fn interrupted_notice_in(state: &AppState, cells: usize) -> Option<String>;

// Unchanged signature: `interrupted_notice(&AppState) -> Option<String>` returns rung 1
// (= `notice_line(.., usize::MAX)`), so every test pinning the full line stays as it is.
```

## Tests (each named at tick time; red first where marked, red run quoted in NOTES)
- **L1** `update::undo::tests::set_aside::the_notice_ladder_keeps_the_clause_and_key` (unit, on `notice_line`): every
  row of the table above, at its rung-1 width, its rung-2 width and 79 cells; rungs 1 and 2 exact; the 46-cell
  description's rung 3 exact at 79; each result's width ≤ `cells`; the clause and `J to inspect` whole in every
  result at or above rung 4's width.
- **L2** `…::the_notice_below_its_last_rung_is_the_last_rung` (unit): 60 cells → rung 4 (the footer cuts it).
- **L3** `…::the_notice_repicks_on_resize` (unit, on `interrupted_notice_in` with one `AppState`): 120 → rung 1,
  then 79 → rung 4, then 120 → rung 1.
- **L4** `…::a_hostile_description_is_measured_as_drawn` (unit): a description with a control character and a
  wide character; the result's display width ≤ `cells` and equals the width of what `sanitize_line` draws.
- **K4** (30c's, extended) `m6_stash_put_back::set_aside::a_relaunch_after_a_kill_before_the_drop_says_the_edits_are_back`:
  at 80x24 the footer row is exactly
  `interrupted · your edits are back · a copy stays in stash@{0} · J to inspect`, and never `edits in stash@{0}`
  alone; 120x40 unchanged (rung 1). **Red first** at 80 (today `… a copy stay…`).
- Stay green unchanged, named at tick time: `update::undo::tests::set_aside::relaunch_after_a_kill_names_the_stash_the_edits_are_in`,
  `…::the_notice_words_where_recovery_found_the_edits`, `m6_stash_put_back::kill::edits_survive_a_kill_at_every_step`,
  `conflict_interim::the_footer_does_not_call_a_live_merge_interrupted`. If any asserts the notice at 80 columns and
  changes, that is the ladder showing: quote old and new in NOTES.

## Acceptance (each tick names its test inline)
- [x] Ladder: L1, L2, L3, L4 green — `update::undo::tests::set_aside::{the_notice_ladder_keeps_the_clause_and_key,
      the_notice_below_its_last_rung_is_the_last_rung, the_notice_repicks_on_resize,
      a_hostile_description_is_measured_as_drawn}`; red first (compile red, NOTES).
- [x] K4 at 80x24 green on the whole rung-4 line (red run quoted) —
      `m6_stash_put_back::set_aside::a_relaunch_after_a_kill_before_the_drop_says_the_edits_are_back`.
- [x] The four regression tests above green, named — `update::undo::tests::set_aside::relaunch_after_a_kill_names_the_stash_the_edits_are_in`,
      `…::the_notice_words_where_recovery_found_the_edits`, `m6_stash_put_back::kill::edits_survive_a_kill_at_every_step`,
      `conflict_interim::the_footer_does_not_call_a_live_merge_interrupted` (all unchanged, full nextest run).
- [x] Capture (K4 test above writes it; both 30c headers rewritten; 1.8 KB / 2.2 KB) `docs/shots/m7/30c-relaunch-edits-back-80x24.txt` regenerated (`GRAFT_UPDATE_SHOTS=1`, then a green
      re-run without it). Its "What right looks like" rewritten: at 80 the bottom row is the rung-4 line, whole. The
      "known gap … cut after `a copy stay…`" lines removed from both 30c captures' headers. Under 200 KB. Colour
      unverified (rule 5).
- [x] Gates, workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`
      (4232/4232), `cargo test --doc`.

## NOTES (builder, 2026-09-27)
- **Red, L1–L4:** before `notice_line`/`interrupted_notice_in` existed: `error[E0425]: cannot find function
  notice_line` (12), `interrupted_notice_in` (3). Compile red, not assertion red.
- **Red, K4** (footer call reverted to unfitted, capture refreshed): `left: " an operation was interrupted: checkout fix
  · your edits are back · a copy stay…"` / `right: " interrupted · your edits are back · a copy stays in stash@{0} · J
  to inspect"`.
- **Table:** every width recounted in the tests (107/90/76, 108/91/77, 78, 57, 112/95, rung 3 = 79). All agree.
- **Decided 5, words:** no plural lead exists. Several entries: `(and n more)` follows the first entry's clause today,
  so with a clause it (and the unreadable warning `its state could not be read, so u restores the whole tree`) are
  part of the never-cut clause; with no clause, `(and n more)` joins the description (cut by rung 3), the warning is
  the clause. Rung 1 is byte-identical to the old line in every case.
- **Measure:** `unicode_width` char sum, as `graft_ui::fit::fit`. `unicode-width` moved from graft's dev-deps to deps
  (same version, already in the lock). graft-ui exposes no public cell-measure fn; not touched (not my crate).
- **Width source:** `footer_notice` gained `columns: u16`. Graph screen passes the frame `area.width`; the stand-in
  footers (`draw_status_or_hints`) pass their row `width`. With the detail pane shown beside the graph, the graph's
  footer may be narrower than `area.width`: the footer's existing `fit` still cuts. Unverified at that layout.
- Below 47–78 cells (rung 4's width): rung 4, cut by the footer. Not laid out further (Decided 2).
- No scout: the ticket named every site; read directly.
- Not compacted.

## Touches (check each, say how)
- `interrupted_notice` callers: `grep -rn "interrupted_notice" crates/graft` — the footer moves to
  `interrupted_notice_in`; tests keep the unfitted one.
- Other lines on this row: `GraphView::problem` and the key hints share the footer; the notice's precedence over them
  is unchanged (no branch touched). `live_entries` still wins (its test above).
- Other screens with the same state: the J browser's `interrupted` row and the graph's `interrupted` `RowMark` read
  neither function (grep); unchanged.
- Captures: `grep -rln "operation was interrupted\|interrupted ·" docs/shots` (planner's grep, 2026-09-27: only the two
  30c files). Any m7 contact sheet holding them → reassemble (`scripts/contact-sheet.sh`).
- Keymap: no key; `J`, `u`, `S p` unchanged (`docs/design/02-keymap.md`).
- Journal/undo: none. The notice reads state only.
- Docs: graft MAP :323 (drop "No ladder…", name `notice_line` and `interrupted_notice_in`, the draw-time pick);
  TESTING.md:2457 (`relaunch-after-kill`: replace "at 80 it is cut after `a copy stay…` — a known gap" with the
  rung-4 line); `docs/design-notes/m3-working-tree-undo-and-stash.md:182` needs no edit (rung 1 is its words).

## For the conductor (not the builder)
- On merge: close the 80-column gap from 30c (K4's open box; tick it in 30c's file "(closed in 30d)").
- **A2 ruling: a defect, not acceptable.** Minor, rare. Not 30d's (graft-ops, the put-back).
  - The line: 30c NOTES "A2", `30c-put-back-review-fixes.md:314-318`, measured by
    `put_back_index::put_back_leaves_a_staged_new_file_deleted_since_unnamed`: before, `n.txt` staged new and deleted
    from disk (`AD`); after the set-aside, `A  n.txt`, on disk at v1. His deletion is undone.
  - Why a defect: (1) the deletion is an edit, and the done line says `your edits are back` with no word for it
    (30b Decided 3, `30b-put-back-words.md:43-44`): a false status. (2) 30b's standard is "the staging now comes back
    as he left it, as `git switch` carries it" (`30b-put-back-words.md:24`); the result is not the state he left.
    (3) The next commit can differ: a stage-all before would have staged the deletion and left `n.txt` out; now it
    goes in.
  - Why minor: no content is lost (v1 is in the index and on disk, as staged); `u` restores exactly (30c NOTES
    :318); a hand-run `stash`, `switch`, `pop` does the same.
  - **Unverified:** that `git switch` alone keeps `AD` as `AD` (my reasoning, not probed), and the `--index` path's
    behaviour (30c NOTES :317). Probe both first.
  - Fix shape for a later graft-ops ticket: `S` can't tell `AD` from `A ` (`S:n.txt == S^2:n.txt` in both), so the
    `AD` paths must be read before the stash (from the status the set-aside already reads) and carried in memory to
    the put-back. Then either name them (`· n.txt came back: you had deleted it`) or delete the file again (its
    content stays in the index). Needs a decision; a kill in between loses the fact (no journal field, 30c Decided 3).
  - File as KNOWN_GAPS **broken**, M8, citing this line.

## Gates
Workspace-wide, quiet: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Own worktree, own `CARGO_TARGET_DIR`, one cargo command at a time.

## Size
About 25k: re-read 4k; `notice_line` and L1–L4 10k; K4, the capture and headers 6k; docs 3k.
