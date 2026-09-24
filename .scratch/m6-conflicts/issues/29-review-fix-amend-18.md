# review-fix-amend-18: a false tick on the landing test; a box opened mid-merge turns the stop into a refusal; old-landing names

Status: ready
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
- [ ] C1: cursor `Some(0)` and `file_cursor` = first file's listed position (non-zero in the fixture), after the read and set land — test:
- [ ] C1: ticket 18 box 1 names the tests that check each of its claims — (textual; line)
- [ ] M1: failing test first, red on unchanged code — test:
- [ ] M1: a box open when the stop arrives → no refusal words, no resolver, the box untouched; comment corrected — test:
- [ ] M2: renames done — (list them)
- [ ] Minor 1: banner asserts restored at 80/120/187; ticket 12 box fixed — test:
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
