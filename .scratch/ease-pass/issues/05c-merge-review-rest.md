# 05c — merge-review rest: the live clash bar, Q7 pause, Q17 landing, relaunch file states

Status: done except `:75`'s `edit`-stop half (a spec contradiction, below) — 05b now 14/15 (2026-10-18, 05c builder) (was ready: conductor split, 05b compacted 3× and stopped at 8/15)
Blocked by: 05b-merge-review-at-y (merged 4ed45107)
Build with: Fable — 05b's own tier; the remainder (landing every clash stop, relaunch classification) keeps 05b's three Fable reasons.
Spec: **`05b-merge-review-at-y.md` is the spec.** Its Decided, Scope, Signatures and Touches hold unchanged. Read its NOTES and Handoff first; they say where each open piece starts.

## Scope = 05b's open acceptance lines
Tick them **in 05b's file**, each naming its test:
- `:63` merge all-solved bar (the live resolver bar must pick `HintContext::Clash`; today it still draws `⏎ done … d start over`). **First**: in a merge `⏎` marks nothing, so today's bar misleads.
- `:64` fixture merge, one file, 2 clashes, landing + `o o` → `Solved`, `ls-files -u` still lists it.
- `:68` journey `m y o y y` lands on the graph at the merge commit (needs Q17's landing).
- `:69` Q7 `esc` pause / banner `⏎` back to the first open clash, picks kept.
- `:71` relaunch `file_state` (binary and delete/modify read `Open`).
- `:74` Q17 a: rebase and cherry-pick clash stops land on the resolver.
- `:75` `edit` stop lands on the graph; `u` mid-merge refuses with §2.6a's words.
- Also (05b report): a solved merge file's row still reads `⏎ marks done` — make it follow 05b's Decided.

Out: 11's clash-screen look, 12's review look.

## NOTES (05c builder, 2026-10-18)

**Context compacted once in this session.** Nothing left over from the compaction: everything after it is in this file and 05b's ticks.

Scouts: two (`scout`), before the compaction: the resolver bar/row sites and the stop/landing sites.

Built (ticks in 05b's file name the tests):
- **Live merge bar** (first): `view/rows/clash_bars.rs` (new) picks `Clash { FileSolved { next } | AllSolved }` for a merge's file with no clash open; `view/resolver/file_list.rs` (new) reads it `FileState::Solved` (graft-ui, `solved ✓`, muted). `finish_owed` / `finish_if_owed` removed.
- **Q7**: `Picks::resolver_opened`/`resolver_seen` (set on every resolver open, cleared at each stop) → `BannerPaused`. Banner `⏎` → `resolver::seek` (new): opens git's first conflicted file and, once its read and the set's land, hands on past solved/done files to the first open clash. A key or click in the resolver cancels the hand-on (`seek::he_acted`) — without it a pick made before the set read landed made the resolver jump files under him (`resolver::last_file::last_file_closes_to_where_it_opened` caught it).
- **Relaunch**: `ConflictSet::solved` (graft-ops, `file_state` per unmerged path on disk); the list reads such a file `AllTaken` unopened.
- graft-ui `paused_banner_list`: was `banner_list(false, …)`, which dropped `s skip` from a paused rebase's bar; now `!merge`, as the unpaused banner's `skippable`.

Contradictions / for the conductor:
- **`:75` not ticked**: the `edit` stop lands with the **graph** focused (`operate/outcome.rs:209-212`); 05b Decided says "as today" (= that), the acceptance line says "banner focused". No code changed.
- Q17 a was already built before 05c (`operate/outcome.rs:224`, `resume/answers.rs:74`); 05c only tests it. `file_state` and its table test were built in 05b.
- Banner `⏎` lives in `banner/focus_keys.rs:126`, not `pressed.rs:71`.
- **Visible cost of the paused words** (by eye, rule 6): `⏎ back to the clashes` is 12 cells longer than `⏎ resolve`. The stop row's rule (`graft_ui` `stopped_row`: the keys never give) then drops stop words first: at 80×24 the paused merge bar has **no `merge stopped`** (header still says `MERGING`); at 120 a rebase's step part goes; at 80 the hint ladder bares `⇥`/`q`. I tried letting hints fall to their short words (`⏎ back`) before the reason drops; it broke graft-ui's `stopped_row_keeps_the_bar_whole` — a design rule — so reverted. Needs a ruling if he wants `merge stopped` kept at 80.

Captures regenerated (what right looks like in each header): `docs/shots/m5-ui-banner-conflict-{80x24,120x40,160x48}.txt`, `docs/shots/m8-banner-bar-q-{187x50,80x24}.txt` (paused bar; 80x24 header line reworded), plus 05c's first commit's six under `docs/shots/m6/`. Colour unverified (rule 5); look unjudged (rule 6). `fix-finish-row` still misnamed.

Tests whose expectation moved to the paused words: `update::banner::tests::{esc_from_a_landed_resolver_shows_the_banner_keys, a_stop_keeps_the_banner_keys_on_the_bottom_row}`, it `conflict_interim::landing_shots::esc_from_the_landing_shows_the_banner_row_on_a_real_merge`, `conflict_interim::capture_banner_bar_with_q`, `m5_walk` `hints-at-80-show-three-and-more`, `m6_stash_put_back::set_aside::a_stop_with_edits_aside_shows_in_the_stop_line_s_and_j` (step now asserted at 187 only), `m8_walk::teaches::resolve_is_on_the_stop_banner_bar`.

Touches: resolver open (`open.rs`), banner `⏎` (`focus_keys.rs`), every answer's tail (`answer.rs` → `seek_first_open`, state-only), resolver key/mouse entry (`he_acted`), `listed_files`, `ConflictSet` (all three constructors: `operation_conflicts`, `conflicts_of`, test `set_of`), `HintContext` picking (`dead_keys`, `sweep_screens`), banner captures, MAP.md ×3. Checked by the workspace gates; keymap untouched (no new key).

Gaps — **broken**: none known. **Unverified**: colour of `solved ✓` (muted) and the paused bar; the look of the 80×24 paused bar (above).
