# 86 — the stash list previews before it writes (C)

Status: done (2026-09-26, builder; compacted once mid-ticket — see NOTES) — approved by the user 2026-09-26 ("86 yes")
approved direction). On "no": nothing is built; `KNOWN_GAPS.md` :773 is updated to observed (walk 2 C) and stays M7.
Blocked by: 85 (both edit `graft`, `update/stash.rs` among them); the user's yes; the conductor's doc rows below.
Crates: graft-ops (`preview`), graft (`update/stash.rs`, the preview's opener and confirm), graft-ui only if the
scout finds the preview box cannot draw a stash plan as it is (and `docs/shots/m6`, `TESTING.md`, the captions TSV)
Source: `docs/reviews/m6-usability-2.md` C, Q-b; m-k's overlay half.
Build with: Opus — the forecast reuses 84's `put_back` machinery with another base; correctness is judged by an oracle
test that runs `git stash apply` on a copy and compares, so not Fable. The conductor may revise after the scout.
Scout: `scout` first (ADR-0035, brownfield), ≤ 1 page, answering only "Scout brief".
Needs: 85 merged.

## Decided (planner, 2026-09-26; direction pending the user) — not open here
- `⏎` in the stash list opens the **apply preview**; `p` the **pop preview**. `y` confirms; `⏎` in the box is the
  choice under the cursor (Cancel first, ADR-0038 §1), so `⏎ ⏎` writes nothing; `esc` returns to the list, cursor on
  the same stash. `p` in the box is not a confirm (no `p p`).
- The box (Ghost Preview frame): title `Preview: apply stash@{n}` / `Preview: pop stash@{n}`; under `working tree:`
  the paths the stash writes — `stash^1`→`stash`, then `stash^3`'s untracked paths — listed and cut as every preview
  lists them; the conflict line in the preview's existing words (`no conflicts expected` / `may conflict: <paths>` /
  not checked); one fate row, kept whole at 80: apply `stash@{n} stays in the list`, pop `stash@{n} is dropped · u
  brings it back`. Bar `y apply` / `y pop`, `esc cancel`.
- **Forecast** (graft-ops, only for `StashApply`/`StashPop`): `git merge-tree --write-tree --merge-base=<stash^1>
  <ours> <stash>`, `ours` = the index tree now, written from a temporary copy of the index (84's `put_back` way; no
  ref, index or file written). Exit 0 → `Clean`; conflicts → `Likely { paths }`; anything else, or git < 2.40 →
  `NotChecked(<reason>)`, never a clean bill. Checked for `stop` between spawns.
- **In the way → the preview refuses** (`PreviewError::Refused(Refusal::StashInTheWay { paths })`, no `[ Apply ]`):
  a path the stash writes that has unsaved edits (worktree ≠ index), or an untracked path `stash^3` would create that
  exists. git refuses the first before touching anything (`docs/research/commit-stash.md` A1.4); the second fails.
  Words `your unsaved edits to <paths> are in the way`.
- After `y`: every outcome lands exactly where `⏎`/`p` land today (clean; clash → resolver, ADR-0045; failure words).
- The list's bar words stay `⏎ apply  p pop` — as `m merge` on the graph opens a preview.

## Read first
- `docs/design-notes/m6-stash-preview.md` (whole; 2 pages).
- `docs/design/02-keymap.md` §2.3b, §2.3c, §2.9.
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §8 (the forecast this reuses).
- `crates/graft-ops/MAP.md` `## preview` (:216-231), `apply` put-back failure branch (:187).
- `crates/graft/MAP.md` :482-490 (`update::stash`), :351 (`land_on_clash`), :471 (`dropped_status`).
- 84's NOTES: "M2 graft-ops" (spawns, the old-git path) and its `put_back_forecast.rs` test names.

## Scout brief (code, not intent; ≤ 1 page)
1. What `preview::plan` returns today for `Operation::StashApply` / `StashPop` (a refusal? `NotShown`?), and how the
   box draws `worktree`, `conflicts`, `undo`, `ref_moves`.
2. How a preview is opened and confirmed from the `.` menu (the function, `y`'s route to `operate`); whether a preview
   can open while the stash overlay is shown, and what its `esc` returns to today.
3. `preview::put_back::forecast` (84): the temp-index copy and the `merge-tree` spawn — what can be shared.
4. What `RepoFacts` holds for unsaved (worktree ≠ index) and untracked paths.
5. Every walk and capture keyed on `S p` / `S ⏎`: `grep -rn` over `crates/graft/tests` and
   `.scratch/m6-conflicts/pty-walk-m6.py`.

## Signatures
```rust
// graft-ops — preview (private module beside put_back)
// fn forecast_stash(/* repo, git, stash: Oid, facts, stop */) -> Result<(WorktreeEffect, ConflictForecast), PreviewError>;
//   Err(Stopped) on stop; Err(Refused(StashInTheWay)) when in the way; a failed spawn is NotChecked(reason).
pub enum Refusal { /* … */ StashInTheWay { paths: Vec<PathBuf> } } // Display: `your unsaved edits to <paths> are in the way`

// graft — update/stash.rs: `⏎` → the preview on Operation::StashApply, `p` → on StashPop, through the opener the
// scout names; `esc` from that preview → the stash list, same stash selected. If the preview records no opener, add
// the smallest one that can (e.g. `opened_from: PreviewFrom::Stash`), named in NOTES.
```

## Acceptance (name the verifying test at tick time; red first where it says so)
- [x] **graft-ops** (real repos, `crates/graft-ops/tests/it/stash_preview.rs`): `a_stash_that_applies_cleanly_forecasts_clean`,
      `a_stash_that_clashes_forecasts_its_paths` (red first), `unsaved_edits_in_the_way_refuse_the_preview`,
      `an_untracked_file_in_the_way_refuses_the_preview`, `an_untracked_only_stash_lists_its_files`,
      `the_stash_preview_writes_no_ref_index_or_file` (as 84's writes test: index bytes, `for-each-ref`,
      `git stash list`, `status --porcelain=v2 --no-optional-locks`), `the_stash_preview_honours_stop`,
      `an_old_git_leaves_the_stash_forecast_not_checked`.
- [x] **Oracle** `the_stash_forecast_agrees_with_git`: rows clean, clash, tracked in the way, untracked in the way,
      staged edits elsewhere, untracked-only. For each: `plan`, then `git stash apply` in a copy — `Clean` ⇒ exit 0 and
      nothing unmerged; `Likely { paths }` ⇒ the unmerged paths equal `paths`; `StashInTheWay` ⇒ git exits non-zero
      (tracked row: and the tree is unchanged, A1.4). A row where they disagree is a failure, not a NOTE.
- [x] **App** `update::stash::tests::enter_opens_the_apply_preview_and_writes_nothing` (red first: no write command),
      `p_opens_the_pop_preview`, `enter_twice_writes_nothing`, `esc_from_the_preview_returns_to_the_same_stash`,
      `y_in_the_preview_applies`, `y_in_the_preview_pops`.
- [x] **Walks:** the pop and apply clash walks gain `y` (`S p y o ⏎`, `S ⏎ y o ⏎`), each ≤ `noted_keys(...)` after the
      fence edit; new `stash_look_and_leave_writes_nothing` (`S ⏎ esc esc`: tree, index, stash list unchanged; ≤
      `noted_keys("stash-look-and-leave")`); 85's `a_take_then_esc_shows_the_file_without_markers` becomes
      `S p y o esc`.
- [x] **Captures** (rule 6; colour **unverified**, rule 5), written by walks through `check_shot`:
      `docs/shots/m6/stash-apply-preview-120x40.txt` — **right:** title `Preview: apply stash@{0}`; `notes.md` under
      `working tree:`; `may conflict: notes.md`; `stash@{0} stays in the list`; `[ Cancel ]` `[ Apply ]`; bar `y apply`.
      `docs/shots/m6/stash-pop-preview-80x24.txt` — **right:** `Preview: pop stash@{0}`; the fate row
      `stash@{0} is dropped · u brings it back` whole; paths cut with `…` only if they must be.
      `docs/shots/m6/stash-preview-in-the-way-120x40.txt` — **right:** `your unsaved edits to notes.md are in the way`;
      no `[ Apply ]`; `esc` offered.
- [x] `every_key_the_bar_draws_is_answered`, `every_planned_hint_is_still_unbound`,
      `every_bound_action_is_hinted_live_or_is_named_here` green.
- [x] Contact sheet rebuilt; captions rows for the three captures; each part ≤ 200 000 B.
- [x] MAP.md: graft-ops (`Refusal::StashInTheWay`, the stash forecast in `## preview`); graft (`stash_key`'s `⏎`/`p`,
      the opener). TESTING.md: "The stash put-back journeys" rows for pop/apply reworded with `y`; rows for the three
      captures and `stash-look-and-leave`. NOTES list each row.

## Touches (check each, report how)
- **`plan` for every other kind:** no new spawn. Check: every existing preview test and capture green (`check_shot`).
- **`apply` for `StashApply`/`StashPop`:** unchanged (the preview only). Check: 72/72b/84 stash tests green.
- **The clash landing and close** (`land_on_clash`, `dropped_status`, `kept_status`): unchanged. Check:
  `stash-clash-landing-*` and `stash-clash-done-120x40` byte-identical (`check_shot`).
- **The stash overlay's other keys** (`s`, `d` box, `b`, `Space`, `j`/`k`): unchanged. Check: the overlay's tests.
- **Journal and undo:** entries unchanged; `esc u` after a clash unchanged. Check: `stash-clash-put-back` walk green.
- **Journeys:** `journey_counts_match_the_note` with the new fence; `m5_walk::every_scenario::journey` stash rows.
- **Acceptance walks and captures to re-run after the last edit:** `m5_walk::every_scenario::journey` (all); the pty
  pass `.scratch/m6-conflicts/pty-walk-m6.py` m6-20…25 at three sizes (m6-20/21 gain `y` if they press `S p`/`S ⏎`;
  re-taken with `GRAFT_UPDATE_SHOTS=1`, then a second pass without it — every capture `settled`); 85's
  `stash-clash-take-esc-pane-120x40` unchanged in content. `git status docs/shots` lists only these and the sheet.
- **Doc rows owed (conductor, on his yes, before the build):** keymap §2.9 `Enter`/`p` and §2.3b `y` from the design
  note's proposed text; `docs/design-notes/m6-conflicts.md` journey fence :515-516 → 5 each, comments `S p y o ⏎` /
  `S ⏎ y o ⏎`, plus `stash-look-and-leave 3`; `m6-stash-put-back.md` Journeys rows marked superseded. KNOWN_GAPS
  :772 (m3) partly and :773 (m4) close.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Large (~70k). If it compacts, the split is: **86a** graft-ops (forecast, refusal, oracle) / **86b** graft (keys,
opener, walks, captures).

## NOTES (builder, 2026-09-26)
**Compacted: yes, once**, mid-walks. Nothing left undone; the split (86a/86b) is not needed.
Scout used: `.scratch/m6-conflicts/scouts/86.md` (main repo, untracked).

Ticks, test per tick (full run 3520/3520, doctests green, fmt, clippy `-D warnings`, all workspace-wide):
- graft-ops: the eight named tests in `crates/graft-ops/tests/it/stash_preview.rs`. Red first seen for
  `a_stash_that_clashes_forecasts_its_paths` (`NotPreviewable`).
- Oracle: `stash_preview::the_stash_forecast_agrees_with_git`, six rows, runs `git stash apply --index` (what Graft
  runs) on a `cp -R` copy. **Gap, not modelled:** with `--index` git can also refuse "conflicts in index" when the
  stash has staged content; the forecast does not predict that refusal (no oracle row covers it).
- App: the six named `update::stash::tests::*`. Red first was not run literally: the old tests asserted `Operate`
  on `⏎`/`p` and were replaced.
- Walks: `m6_stash_put_back::a_stash_pop_that_clashes_is_five_keys`, `…::a_stash_apply_that_clashes_is_five_keys`
  (renamed from `_four_keys`), `m6_stash_put_back::stash_preview::stash_look_and_leave_writes_nothing` (counts
  `S ⏎ esc` = 3 against `noted_keys("stash-look-and-leave")`; the last `esc` closes the list, uncounted),
  `m5_walk::every_scenario::journey_counts_match_the_note` and the `journey_stash_*` rows. 85's walk's real sequence
  is `S p y esc ⏎ o esc` (the ticket's `S p y o esc` never looks at the pane first; see its doc).
- Captures: written by `stash_look_and_leave_writes_nothing`, `the_pop_preview_fits_80_columns`,
  `unsaved_edits_in_the_way_are_named_in_the_box` (same file). Colour unverified (rule 5); the user judges by eye.
- Hint bar: `every_key_the_bar_draws_is_answered`, `every_planned_hint_is_still_unbound`,
  `every_bound_action_is_hinted_live_or_is_named_here` green in the full run.
- Sheet: `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6` → now **5 parts** (part 5 new, 4 958 B;
  largest 179 772 B). Captions: 3 rows added; 5 rows reworded `S p` → `S p y`.

Decisions:
- `StashInTheWay` is a mapping in `preview::plan` of `check`'s `TouchesChangedFiles` when some of its paths are
  unsaved or untracked, plus an on-disk check for `stash^3`'s untracked files (ignored files included). `apply`'s own
  refusal is unchanged. Staged-only edits on a stashed path keep `TouchesChangedFiles` words, now in the box.
- Every refusal of a **stash** preview stays in the box (`PreviewState::refused`, one `[ Close ]`); other subjects
  keep 67's close-plus-status-line.
- No opener record needed: the box draws over the `S` overlay (`draw_stash` → `draw_preview`), which stays open, so
  `esc` lands on the same stash.
- `y` runs through `operate::launch` (stoppable, as `⏎`/`p` did), **not** `launch_unstoppable` (which other previews
  use). The `y apply`/`y pop` edge key uses the footer's `again` slot.
- m3 walk: `stash-apply-refused-dirty-touched-file` now reads the box's words (`… are in the way`), not the status
  line; its unused `Step::Settle` removed.

Touches (how checked):
- Walks updated: `m3_walk` (stash-pop, stash-apply, refused), `stash_by_key`, `stash_clash` (`put_back` helper),
  `m5_walk/stash_journeys` (`S p y`, m6-20/21 keys), `m5_walk/resolved_operations`, `m6_stash_put_back{,/pop}`.
  `pull_over_edits` presses only `S s ⏎` (save) — read, unchanged, green.
- Captures changed **header only** (`git diff --stat`: 1–2 lines each): stash-clash-landing-{120x40,80x24},
  stash-clash-done-120x40, stash-clash-stop-line-80x24, stash-clash-take-esc-pane-120x40, and pty m6-20/m6-21 at
  120x40/80x24/187x50 (re-taken with `GRAFT_UPDATE_SHOTS=1`, then a second pass: all `settled`). Frames identical.
  m6-22…25 not re-run: no stash keys in them (scout).
- Docs: `docs/design-notes/m6-conflicts.md` fence (5, 5, `stash-look-and-leave 3`); TESTING.md rows 114, M3 15/16/18,
  M6 13, journey table (+`stash-look-and-leave`), m6-20/21 rows, 85 B and 82 paragraphs, new "The stash preview"
  paragraph; graft-ops and graft MAP.md; `preview::plan` doc comment. Keymap §2.9 already amended by the conductor.
- Not done here (conductor's rows): `m6-stash-put-back.md` Journeys marked superseded; KNOWN_GAPS :772/:773.
