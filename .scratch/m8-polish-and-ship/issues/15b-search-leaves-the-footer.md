# 15b — Search leaves the footer after ⏎ (split from 15, A2)

Status: done (builder 2026-10-08; see NOTES). Was: ready (conductor 2026-10-08; 15 merged) — split from `15-bar-content-pass.md` along its own seam; no new decision.
Build with: **Opus** (as 15).
Blocked by: 15 (merged first: `Searching` slots and fit order build on its A1/A4/A5).

## Scope

Exactly `15-bar-content-pass.md` **Amendment A2** (from "**A2 — Search**", line 59) and its acceptance boxes:
the three `A2` boxes plus the capture `docs/shots/m8-bar-after-search-120x36.txt`. Read 15's **NOTES (builder,
2026-10-08)** first: it is the handoff (what 15 left and where A2 starts). Scout:
`.scratch/m8-polish-and-ship/scouts/15-bar-content.md` (search path: `rows.rs:325-335`, `footer.rs:86-90`,
`escape.rs:30`, the test at `footer.rs:334` A2 replaces).

Out: everything else in 15 (built).

## Acceptance

Tick the A2 boxes **in 15's ticket file**, each naming its test, and this line:

- [x] All three A2 boxes in `15-bar-content-pass.md` ticked with tests; capture `m8-bar-after-search-120x36.txt`
      written and asserted by a test, ≤200 KB. **Colour unverified.**
      — tests named in 15's three A2 boxes; capture (3.2 KB) asserted by `graft` it
      `search_hint::after_enter_the_bar_is_the_rows_keys_and_the_search_is_in_the_header`.

## NOTES

**Builder, 2026-10-08.** Status: done. Scout used: `.scratch/m8-polish-and-ship/scouts/15-bar-content.md` (search
path traced there; not re-scouted). No compaction.

What was built: `SearchLine::Status` removed; `SearchTail { text, failed }` + `GraphView::search_tail`;
`HintContext::Searching(Box<HintContext>)` (`n next`, `N prev` lead the inner row bar; `catalogue::searching_form`;
coverage slot 114, `COVERAGE_SLOTS` 115); `graft` `view::rows::search_tail` and `search_wrapped` (applied before
`live_wrapped`, so `OperationLive { row: Searching(..) }`); header tail: search leftmost, items drop whole
gfx → walk (with `Esc cancels`, order notice) → filters → search → banner (`layout::fitted_tail`).

**Deviations from the spec — the conductor's call:**
- `HeaderView::search` was **not** added. The tail is fitted in `layout.rs` (`GraphScreen::header`), which already
  owns the walk/filters/gfx/banner items and the banner's ladder; `header_line` only draws a pre-fitted `Line`. Putting
  the search in `HeaderView` would have split one drop order across two files. The search reaches the header via
  `GraphView::search_tail` instead.
- **`n N` lead only while the search has a hit.** With zero hits `n`/`N` step nowhere (`update/search.rs:169`
  `search.step` → `None`), so leading with them would offer dead keys (§2.1 mode rule; `search::keys_words` already
  drops its `n/N` clause at zero hits). Running-with-hits and done-with-hits get `Searching`. Asserted in the it test
  (`/zzqx`: no `n next` in the bar).
- **The last non-search group is cut with `…`, not dropped**, when it alone does not fit (e.g. a lone
  `history failed: <long reason>`): today's "a failure is shortened, never silently dropped while any cell is free".
  The search is whole or absent, per spec. A gfx failure beside other items is now dropped whole first (spec order),
  where before it was cut — a behaviour change at narrow widths.
- Items are joined with two spaces, as the tail already was; the spec's `search · …` was read as notation.
- At a stop beside a row bar, the stop's *reason* still drops when the ladder fills the row (`fitting.rs:421-423`,
  accepted in m6-conflicts/12); the replacement footer test asserts the stop's keys, then `n`, then `N`, and allows
  the reason-or-bar lead.

**Existing tests changed:** `footer.rs` `a_search_line_still_wins_over_a_stop` → replaced by the two named tests;
`view::tests::bottom_row::rungs::the_search_rung_is_blank_under_the_preview` now reads `search_tail` (the standing
search's new home), assertions otherwise the same; `graph_screen::tests::view` got an `#[allow(too_many_lines)]`
(one more struct field). Sweeps extended, not loosened: `update::tests` `classified_form`/`screen_for`/`states_for`
gain `Searching` (`search_standing` puts a two-hit done search on the fixture), so
`every_key_the_bar_draws_is_answered` presses `n`/`N` in every `Searching` form and
`every_swept_state_is_the_context_it_claims` confirms the classifier yields `Searching`.

**Captures:** new `docs/shots/m8-bar-after-search-120x36.txt`. Re-shot `docs/shots/m8/search-hint-120x36.txt`
(search line moved from footer to header in all three frames; header comment extended). **Not re-shot:**
`m2-18-git-git-search-enter-{80x24,160x40}-unicode.txt`, `26-search-far.txt`, `26-search-typing.txt` — hand-shot pty
records of their tickets (`search_hint.rs:3-4`, TESTING.md "Captures"), against the git.git and 100k-synthetic repos;
no test regenerates them. `26-search-typing` shows the open prompt, which is unchanged. The other two now show the
pre-15b footer: stale records, not regenerated here. The ticket's other captures (`m8-help-rebase-stop`,
`m8-banner-bar-q-*`, `m8-quit-*`, `15-stash-hidden`) have no standing search; their tests pass unchanged.

**Touches checked:** `SearchLine::Status` readers — compiler (footer.rs only); `HintContext` exhaustive matches —
compiler (graft-ui hints/catalogue/tests, graft `update::tests`); `live_wrapped`/`back_to_place` — `Searching`
added to both lists (`back_to_place` is moot while a search stands: `esc_goes_back` is false then); palette/help
does not read `HintContext` (15's NOTES grep); keymap: `n`/`N` are `app.next_match`/`app.prev_match`, already bound;
journal/undo: none. Not touched: 18c's files (empty_state, places, jobs/clone).

**Colour unverified:** the search item's Info/Warning styles in the header (captures carry no colour).
