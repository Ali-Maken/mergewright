# 21b-5 additions (planner, from 18e B)

Written to its own file: the planner has no append-only tool here, and rewriting `21b-hygiene-round-2.md` was ruled out.
The conductor folds this into 21b-5's section of `21b-hygiene-round-2.md`, after "21b-5 additions (conductor …)".

**Source:** 18e NOTES "B (builder)" (`.scratch/m8-polish-and-ship/issues/18e-clone-review-fixes.md`). 18e B1 applied
every named seam, and three files are still over 400 lines. These are the seams the builder proposed, with a ruling
on each. All are move-only, and 21b-5's "Move-only check" applies (steps 1–6).

**Crate set added to 21b-5:** graft `empty_state.rs` + new `empty_state/palette.rs`; graft
`event_loop/places/outside.rs` + new `places/outside/carry.rs`; graft-ui `empty_state.rs` + new `empty_state/draw.rs`;
both MAP.md. This is disjoint from 21b-5's other files (`update/**`, and the graft-ui `graph_screen/layout.rs`,
`footer.rs`, `view/rows.rs`, `catalogue.rs`).
**Build with:** Sonnet. The work is move-only and judged by `nextest list` and the line-set check.
**Blocked by:** 21b-4 merged and 18e B merged. 21b-4 item 4 edits `outside_paste`, and item 3 owns the graft
`empty_state.rs` split. **One ruling for both parts:** if 21b-4's builder reaches graft `empty_state.rs` first, it uses
seam 1 below, and box 5i ticks by naming 21b-4's commit.
**Read first:** 18e NOTES "B (builder)" (the counts, and the method: a script, `pub(super)`, re-exports, and
`#[cfg(test)]` re-imports). `crates/graft/MAP.md` row 6 (`empty_state`) and the `places::drive` row.
`crates/graft-ui/MAP.md` `empty_state`. Don't read the files whole: list their `fn`/`struct`/`impl` lines.

## Rulings
1. **graft `empty_state.rs` (510) → `empty_state/palette.rs`: taken as proposed.** Move `act`, `source_of`,
   `palette_rows`, `palette_key`, `outside_paste` and `render_palette` (~170), which leaves ~340. The test file is
   already `palette_tests.rs`, so the code goes beside its tests. `first_runnable` stays wherever 21b-4 item 4 left
   it, and the child imports it.
2. **graft `places/outside.rs` (478) → `places/outside/carry.rs`: proposed as `outside/jobs.rs`, renamed to `carry.rs`.**
   - Why the rename: it mirrors `running/carry.rs` (the repo place's `carry_out`), and it avoids a second `jobs`
     module beside `crate::jobs`.
   - What moves: `OutsideJobs`, `carry_out` and its three steps (`init_and_open`, `start_clone`,
     `end_first_launch`), plus `AnswersAsks` and `answer_ask` (~170). That leaves ~310.
   - `try_open` moves **only if** every caller is in `carry.rs` (check with `rg -n 'try_open' crates/graft/src`, quoted
     in NOTES). Otherwise it stays in `outside.rs`.
   - **Rejected:** sending `try_open`, `read_recents` and `draw_whole` back to `places.rs`. That leaves `outside.rs` at
     ~430, still over.
   - **Visibility:** `places/tests.rs` reaches these items through `places`, one module up from `outside`.
     `pub(super)` in `carry.rs` can't be re-exported that far, so widen to `pub(in crate::event_loop::places)` and no
     further. `places`' existing `#[cfg(test)]` re-imports keep their names, and `places/tests.rs` is not edited.
3. **graft-ui `empty_state.rs` (556) → `empty_state/draw.rs`: taken as proposed.**
   - What moves: `draw_head`, `draw_actions`, `draw_recent`, `draw_prompt`, `draw_field_cursor` and `draw_cursor`
     (~230), as their own `impl EmptyState<'_>` block. This is the precedent `clone_prompt.rs` and `bar.rs` set.
     It leaves ~330, with the view types and the `Widget` impl in the parent.
   - `keep_end` goes with its callers if every one of them moves. Otherwise it stays in the parent as `pub(super)`.
     Say which in NOTES, with the `rg`.

**Stop rule:** if a file is still over 400 after its seam, stop on that file and give the count. Don't add a second
seam: that is another spec question.

## Acceptance (each tick names its check inline)
- [x] 5i (843be1d8, done by 21b-4, NOTES `21b-hygiene-round-2.md:537–538`; `wc -l` at the hash: `empty_state.rs` 373, `empty_state/palette.rs` 149; ticked by 31): graft `empty_state.rs` and `empty_state/palette.rs` are each ≤ 400 by `wc -l` on the committed file (quote
      both). `cargo nextest list --workspace | sort` before and after the commit: `diff` empty (quote the count).
- [x] 5j (31fc7d49; `wc -l` outside.rs 281, outside/carry.rs 223; nextest list diff empty, 6201; the commit's stat lists no places/tests.rs; `try_open` moved: `rg -n try_open crates/graft/src` callers only :349/:372/:386, all in `carry_out`/`init_and_open`): graft `places/outside.rs` and `places/outside/carry.rs` are each ≤ 400 by `wc -l` on the committed file
      (quote both). The `nextest list` diff is empty. `git diff --stat` of the commit doesn't list `places/tests.rs`.
- [x] 5k (2991ed03; `wc -l` graft-ui empty_state.rs 333, empty_state/draw.rs 248; nextest list diff empty; `keep_end` stays in parent: `rg keep_end` shows clone_prompt.rs:8,84): graft-ui `empty_state.rs` and `empty_state/draw.rs` are each ≤ 400 by `wc -l` on the committed file (quote
      both). The `nextest list` diff is empty.
- [x] 5l (one commit each 31fc7d49/2991ed03; line-set check: only use lines, `pub(in crate::event_loop::places)`/`pub(super)` widening, fmt re-wraps, one comment line; docs/shots diff empty; MAP greps hit graft MAP.md:109, graft-ui MAP.md:7; gap rows: `carry_out` now `places/outside/carry.rs:74`, `draw_prompt` graft-ui `empty_state/draw.rs:147`), for every move above:
      - One commit per file.
      - Line-set check, as in 13l A1: only `use`/`mod`/`//!` lines, the visibility widening and `fmt` re-wraps
        differ. List them in NOTES.
      - `git diff --stat <base> -- docs/shots` is empty, with `GRAFT_UPDATE_SHOTS` unset.
      - The MAP layout lines name the new files: `grep -n 'empty_state/palette.rs\|places/outside/carry.rs'
        crates/graft/MAP.md` and `grep -n 'empty_state/draw.rs' crates/graft-ui/MAP.md` each hit a row.

## Touches (check each; say how)
- **Callers:** public paths stay the same through re-exports, so the workspace compiles with no `use` change outside
  the split files (`git diff --stat` shows it). The test files `empty_state/{tests,palette_tests,sweep_tests}.rs`,
  `places/tests.rs` and graft-ui `empty_state` tests are **not edited**.
- **Captures** must pass unchanged: `docs/shots/m8/first-launch-*`, the empty-state and clone captures (`rg -l
  'empty_state|clone' crates/graft/tests/it/m8_walk`).
- **Gap rows citing moved lines** (for the conductor): M8/clone-open-fails-stuck (`carry_out`, failed open) and
  M8/clone-long-fns (`carry_out`, `draw_prompt`). NOTES gives each one's new file:line.
- **18e's B1 box** ticks by naming 5i–5k. With it close the graft `empty_state.rs` part of M8/11-files-over-size and
  the graft-ui `empty_state.rs` part of M8/look-file-sizes.
- **Not touched:** keymap, journal, undo, graft-ops, ARCHITECTURE.md.
