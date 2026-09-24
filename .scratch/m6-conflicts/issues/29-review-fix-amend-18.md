# review-fix-amend-18: a false tick on the landing test; a box opened mid-merge turns the stop into a refusal; old-landing names

Status: done
Blocked by: 28-done-file-start-over-only-where-it-can.md (building; same crate, ADR-0036). Runs before 16.
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-18.md` Critical 1, Major 1, Major 2, Minor 1.
Build with: **Opus** (low).
Needs: none.
Crates: graft.

## Read first
- `docs/reviews/m6-amend-18.md`: Critical 1, Major 1, Major 2, Minor 1 — each names file and line.
- `.scratch/m6-conflicts/issues/18-stop-opens-the-resolver.md`: Acceptance line 1, NOTES.
- `crates/graft/MAP.md` (`update::banner::stop`, `update::resolver`, `commit_editor`).

## Scope
In:
1. **Critical 1 — false tick.** Ticket 18's box 1 claims the test checks the cursor on block 0 and `file_cursor` = that file's index; `a_conflicted_merge_lands_in_the_resolver` (`update/operate.rs:2235`, `:2254`) asserts `cursor_block == None` and a `file_cursor` of 0 that any path passes. Add an integration test that drives the stop on the real two-file fixture, waits for the status read and the set, then asserts `cursor_block == Some(0)` and `file_cursor == position of first_conflicted in listed_files` (use a fixture where that position is **not** 0, so the assert can fail). Then correct ticket 18's box 1 to name the test that checks each claim.
2. **Major 1.** A box opened after the launch (`c` → commit editor, `update/keys.rs:249-251`, no `operating` guard) turns the stop into `finish the box first` nobody asked for, and the P5 landing is lost (`update/banner/stop.rs:37-46`; its comment says this can't happen). Failing test first (the review's probe: `operating = Some(_)`, `c`, then `Conflicted`). **Conductor's ruling:** `land_in_resolver` returns early when `resolver::box_open(state)` — the stop lands where it lands today without a resolver, and **no** refusal words appear. Correct the comment.
3. **Major 2.** Test names, messages and the `m5-06` capture name that still say the stop lands on the banner (review lists them) → renamed to what they check now.
4. **Minor 1.** `capture_a_merge_that_stops` (`tests/it/conflict_interim/landing_shots.rs:49-85`) lost its real-repo banner asserts. Restore them on the `Esc`'d state from `stopped_on_the_conflict` at 80, 120 and 187: `⏎ resolve  c continue  a abort`, no `esc dismiss`, `merge stopped · 2 files conflicted ·` at ≥120. Fix ticket 12's box (`12-landing-after-a-stop.md:127`) to name live files and this test.

Out: review-18 Minors 2–7 (→ KNOWN_GAPS).

## Acceptance (each tick names its test, at tick time)
- [x] C1: cursor `Some(0)` and `file_cursor` = first file's listed position (row 1: `alpha.md`, conflicted then resolved+staged by rerere, heads the list as done; the test asserts `at != 0`), after the read and set land — test: `it::conflict_interim::the_landing_puts_both_cursors_on_the_first_conflicted_file`
- [x] C1: ticket 18 box 1 names the tests that check each of its claims — `18-stop-opens-the-resolver.md:34`
- [x] M1: failing test first, red on unchanged code (`Sticky { reason: "finish the box first" }` vs `Ready`) — test: `update::operate::tests::a_box_opened_mid_merge_keeps_the_stop_on_the_banner_without_words`
- [x] M1: a box open when the stop arrives → no refusal words, no resolver, the box untouched; comment corrected (`update/banner/stop.rs` `land_in_resolver`) — test: same as above
- [x] M2: renames done — `conflict_focuses_the_banner_and_lists_the_files` → `esc_from_the_landing_shows_the_banner_and_lists_the_files` (+ message); m5_walk `conflict_focuses_banner_and_lists_files` / `conflict-focuses-banner-and-lists-files` → `stop_then_esc_lists_files` / `stop-then-esc-lists-files` (+ See text); capture stem `m5-06-banner-conflicts` → `m5-06-stop-lands-in-resolver` (3 files, TESTING.md:2019, coverage.rs, 35-coverage-matrix, contact sheet, captions TSV keys)
- [x] Minor 1: banner asserts restored at 80/120/187; ticket 12 box fixed (`12-landing-after-a-stop.md:127`) — test: `it::conflict_interim::landing_shots::esc_from_the_landing_shows_the_banner_row_on_a_real_merge`
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest 2872/2872 (1 skipped), doctests pass

## NOTES
- No scout: the review names every line.
- Scenario name chose `stop-then-esc-lists-files`, not the resolver name: its See runs after `merge_that_stops`' `Esc`, on the banner. The capture (keys `m ⏎`) shows the resolver, hence stem `m5-06-stop-lands-in-resolver`.
- Captions TSV rows 10-12 keys renamed only; their stale wording is review Minor 3 (out of scope).
- `resolver::box_open` widened to `pub(in crate::update)`; not public, MAP.md unchanged.
- No compaction.
