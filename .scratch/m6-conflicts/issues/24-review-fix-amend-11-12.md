# review-fix-amend-11-12: count cut mid-number at 187 cols, banner.rs split, doc drift, dead `y`, silent landing drops

Status: done
Blocked by: none (11, 12, 23 merged)
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-11-12.md` M1, M2, M3, Minor 1, Minor 3. Minor 1 restores `02-keymap.md:111` ("a key the menu displays must actually work"), which outranks 11's scope item 4 — conductor's ruling.
Build with: **Opus** (medium).
Needs: none.
Crates: graft, graft-ui.

## Read first
- `docs/reviews/m6-amend-11-12.md`: M1, M2, M3, Minor 1, Minor 3 — each names file and line (note: `hints.rs` was split by ticket 23 after that review; `stopped_row` now lives under `graft-ui/src/graph_screen/hints/`).
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`.
- `ARCHITECTURE.md` before moving modules; `codebase-design` for the split.

## Scope
In:
1. **M1.** `stopped_row`'s `fit(reason, room)` cuts characters from the end, so 13 files reads `1…` at 187 cols with the pane open. Cut the reason at `·` boundaries: drop whole parts, lowest value first — `begun outside Graft`, then `from a previous run`, then the step. The count part is kept whole or dropped whole, never cut inside. Test at 108 cells with 13 files. Recapture `docs/shots/m6/fix-landing-b-launch-187x50.txt`.
2. **M2.** Move the stop/landing concern out of `update/banner.rs` (`:574-793`: `stopped_again`, `finished_lands_on_head`, `refs_answered`, `walk_started`, `land_on_head`, `stop_reason`, `stop_line`, `one_tab_to_banner`) into `update/banner/stop.rs` or `update/relaunch.rs` — your call, say why. Pure move. `preview.rs`, `state.rs`, `rows.rs`: not in scope — the conductor adds a size row.
3. **M3.** `update/mod.rs:539-542` doc comment back above `took_fact`; `crates/graft/MAP.md:224` corrected (`conflicted_line` gone; `Conflicted` sets `AppState::stopped`); `tests/it/conflict_interim.rs` `capture_banner_for_docs_shots` dead `Tab` round trip removed (re-check the shot unchanged) or its purpose stated.
4. **Minor 1.** `update/preview.rs:459-468` `footer_keys`: set `confirm_key`/`again` only when the plan `acts()` or is `planning()`. Test: a `closing_box` and a can't-act plan draw no `y confirm`; `every_key_the_bar_draws_is_answered` (or equivalent) covers them.
5. **Minor 3.** `banner.rs:717-719`, `:721-723` (moved by item 2): each silent `return` either gets a comment saying why dropping is safe, or stops being silent. Then answer the review's question with a test: `Ctrl+R` straight after `c` (a second refresh replacing the walk before it starts) — is `land_on_head` left owed forever? If the test shows it is, fix at the cause (point `head_landing.walk` at the newest walk, or drop the landing when `awaiting` changes). If it shows it isn't, say so in NOTES with the test name.

Out: every other Minor (they go to KNOWN_GAPS); ticket 13's work.

## Acceptance (each tick names its test, at tick time)
- [x] M1: 13 files at 108 cells keeps the count whole or drops it whole — test: `graft-ui` `graph_screen::hints::tests::stopped_row_keeps_the_count_whole_at_108_cells` (108 exact, plus every width 60..=200), `stopped_row_cuts_only_the_last_wording`; ladder: `graft` `update::banner::tests::stop_wordings_drop_whole_parts_lowest_value_first`
- [x] M1: recaptured 187x50 shows the count — `docs/shots/m6/fix-landing-b-launch-187x50.txt` last row ` merge in progress · 13 files conflicted · ⏎ resolve …`, pinned by `conflict_interim::landing_shots::capture_a_launch_into_a_shell_merge` (asserts the row starts `merge in progress · 13 files conflicted ·` at 120 and 187)
- [x] M2: stop/landing code moved; `banner.rs` line count in NOTES — (gates green; pure-move commit `a20ec55`)
- [x] M3: three drift fixes — `update/mod.rs` `took_fact`'s two doc lines back above it (`working_tree_read` now opens with its own); `crates/graft/MAP.md` `operated` entry: `Conflicted` sets `AppState::stopped`, `Status::Ready` unless git stalled (`stalled_line`); `tests/it/conflict_interim.rs` `capture_banner_for_docs_shots`: dead `Tab` round trip gone (`on_the_graph = state.clone()`), the remaining `Tab` given its reason (the modal banner would swallow the `k`s). Shots re-checked: of the three `m5-ui-banner-conflict-*` files, 120x40 is unchanged and 80x24, 160x48 differ on the footer row only, by item 1's count change
- [x] Minor 1: no `y confirm` on a closing box or a can't-act plan — test: `update::preview::tests::y_confirm_is_drawn_only_where_y_acts` (failed, undrawable, empty plan: neither key drawn, `y` operates nothing; acting plan: drawn and `y` operates; planning: drawn, waiting). `every_key_the_bar_draws_is_answered` does not reach the box's own footer, so this test is the equivalent
- [x] Minor 3: no silent drop without a reason; Ctrl+R-after-`c` question answered — test: `update::banner::tests::a_refresh_before_the_walk_starts_still_lands_on_head` (was red before the fix, at "spent, not owed for good")
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest (2766 passed, 1 skipped), doctests — all green, run in this worktree with `CARGO_TARGET_DIR=/private/tmp/claude-501/graft-target-24`

## NOTES

No scout: the review names every line; `stopped_row` found at `graft-ui/src/graph_screen/hints/fitting.rs`.

**M1 — cause.** `stopped_row` cut one string from the end, and only `graft` knows which parts matter. Now `graft` writes a ladder (`banner::stop_wordings`: full, then minus origin words, minus the step, minus the count) and `graft-ui`'s new `StopWording` carries it in `GraphView::stopped`; `stopped_row` draws the fullest that fits, and cuts only the last (number-free) one. Cost to the user, visible in the regenerated shots:
- 187x50 and 120x40 launch, pane open: `begun outside Graft` is gone from the footer; the count is whole (`13 files conflicted`). The header still says `MERGING`.
- Relaunch at 120x40 with the pane open: `from a previous run` drops the same way (tests finding the row by it now find it by `… still in progress`).
- 80x24 and 160x48 banner shots: `merge stopped ·…` / `16 files co…` became `merge stopped` — the count dropped whole.
- **Conductor to rule:** the ticket's order drops *whose* operation it is before the step and the count. That is what was asked, but at the user's 187 with the pane open he no longer sees "begun outside Graft" on that row. The J3 row promises the count (`m6-conflicts.md:260`, per the review; not re-read), so I kept to the ticket.
- Not regenerated: `docs/shots/m6/m6-contact-sheet.txt` (acceptance run's job) and TESTING.md scenario 14's lines (Minor 8, already filed).

**M2 — pure move** to `update/banner/stop.rs`, not `relaunch.rs`: the moved code reaches four banner-private items (`banner_focusable`, `STUCK_KEYS`, `live`, `unmerged_paths`); a child module reads them as `super::`, `relaunch.rs` would have made them wider. The landing is not a relaunch concern either. Public paths unchanged (re-exported from `banner`). One visibility narrowed: `land_on_head` was `pub(super)` but only called inside the moved code, so it is private now (clippy flagged the unused re-export). **`banner.rs`: 796 → 640 lines** (`stop.rs` 275 after items 1 and 5 added to it). Still over the ~400 limit — the conductor's size row.

**M3** — textual; see the tick.

**Minor 1 — cause.** `footer_keys` now takes the `PreviewState` and sets `confirm_key`/`again` only when `acts() || planning()`, the same condition `confirm_by_key` answers.

**Minor 3 — answer: yes, reachable, and fixed at the cause.** The test presses `c` (Done), then `Ctrl+R` before the refresh's walk starts: the first walk's `Started` is dropped by `history_started` (`mod.rs:515`), so `walk_started` never matched and `land_on_head` stayed owed. Fix: `walk_started` spends the walk half on **the awaited walk starting**, whichever refresh issued it (every walk that passes `history_started` after `resumed`'s refresh was begun after the `c`). Its `id` parameter went. The two silent `return`s in `land_on_head` now carry comments: no history is unreachable there (the walk half is spent right after `history_started` installs one); no `awaiting` drops the seek request exactly as `keys::move_cursor` does, since no walk is running to answer it.

**Gaps to file (not fixed, out of scope):**
- Unverified: a walk that fails or is cancelled *before* its `Started` (`job_failed`/`job_cancelled` clear `awaiting`) leaves the walk half unspent, so the landing stays owed until the next `c`. Not tested; no line shows it reached in practice.
- Unverified: the landing's refs half still matches by id; a refs read that fails clears it (`Refs(Err)`, per the review — not re-read here), but nothing was tested for a refs read cancelled by shutdown.

Model: Opus. No compaction.
