# M8 review: palette, doorway rows, theme overrides, palette perf

Group: tickets 11, 11b, 16, 12, 12b, 12c. Fixed hash `81075e6f` (main).
Merges diffed against their first parent: `f5b421df` (11), `91f501b0` (11 cont.),
`0a5b9fa5` (11b), `c8b005d9` (16), `57907aba` (12), `684a22a6` (12b), `5650dc40` (12c).
11c/11d (the rest of the doorway) are out of scope; they were already ruled on.

**Counts:** Critical 0 · Major 4 · Minor 11 · Questions 3.

## What was verified, and how

- **No cargo this round** (conductor's CPU cap: two builders running). So none of these ran and all are **unverified**: `cargo fmt --check`, both clippy lines (plain and `--features heavy-tests`), the fast suite, the heavy tier and the release perf run. No fast-suite time, slowest tests, peak memory or `target/` size was measured. The next round that may run cargo has to run the release perf run (ADR-0021). 12c's ADR-0021 box is honestly unticked, and gap `M8/12c-release-run-9-vs-7` stands.
- Everything below comes from reading the diffs, the tests and the captures. A finding that needs a run to confirm says so.
- **Ticks spot-checked: 24, and all 24 held.** Each named test exists and asserts its claim.
  - 11: 6 checked, 6 held: matcher ranks, J4 walk, J9 walk, per-place spelling, config problems listed, loop theme test.
  - 11b: 4/4: hint doorway, fallback, detail-tab first-parent, outside click closes.
  - 12: 5/5: `walk_seek_branch`, shown rows match what runs, nothing-matches row, recent cap, picker order unchanged.
  - 12b: 4/4: not-committed rows, Esc lands, `more` row starts `/`, walks for words and for a file.
  - 12c: 2/2: `row_badge_work` bounded, and the named tests exist.
  - 16: 3/3: background override, re-detect through retheme, selection-on-background problem.
- Unticked boxes are honestly unticked and gapped: 12 perf, 12b perf (`M8/12b-perf-pty-write`, broken), 12c release run.
- **Feature pipeline.** The design note `docs/design-notes/m8-polish-and-ship.md` has Need, Prior art (cites `docs/research/palette-find.md` and others) and Journeys (J4–J9, J26–J32).
  - The walks' step counts match the note's fence: J4, J9, J26–J30 were counted against `crates/graft/tests/it/palette_find.rs` and the J7 test.
  - The help palette was prototyped (scene 2). Find was not; see Q2.
- **Verified clean, by reading:**
  - Escapes are sanitised: `sanitize_line` runs on the query, row labels, paths, subjects and config-problem text.
  - No git runs on the UI thread: names, tags, files and commits are all jobs, stale answers are dropped, and work is cancelled on edit or close.
  - No new `unwrap`/`expect` in production code.
  - No swallowed errors: each `unwrap_or` is commented.
  - The 12c cut is proven: every badge is at least 2 cells plus a separator, so `columns/2+1` bounds what can fit.
  - The theme flush happens only on a real theme change (`event_loop/running/turn.rs` `terminal_changed`).
  - Captures are 3–5 KB each, under the 200 KB cap.

## Critical

None.

## Major

### 1. IN PROGRESS rows run the graph's key, not the banner's: "a Abort" amends

- **Where:** `crates/graft/src/update/palette.rs:1250-1253`. `PaletteTarget::Action(id)` runs `dispatch_key(state, commands, chord.to_press())` whatever context the row was spelled in.
- **Why it misfires:**
  - IN PROGRESS rows are spelled in `ContextId::Banner` (`palette.rs:677-687`).
  - `banner_key` returns `false` unless `state.banner_focus` (`crates/graft/src/update/banner/focus_keys.rs:100`). Only with focus does `s`→skip and `a`→abort (`:115-116`) apply.
  - `?` opened from the Graph during a stop leaves the banner unfocused. The capture `docs/shots/m8-help-rebase-stop-120x36.txt` shows this state: header REBASING, banner unfocused, and rows `c Continue`, `s Skip this commit`, `a Abort`.
  - So `⏎` on "a Abort" reaches the graph's `a`, which is amend (`crates/graft/src/update/keys/commit.rs:26-34`). At an edit stop with the cursor on HEAD, that runs `git commit --amend --no-edit` at once (`commit_editor/edit_stop.rs:13-28`). Elsewhere it opens amend-into.
  - "s Skip this commit" does nothing, because the graph has no `s`.
  - Only `c` works, because the graph routes `c` to continue at a stop.
  - The J7 test (`tests/it/palette_in_progress.rs`) checks only `c`. The builder's NOTES say "s/a on the graph: not checked".
- **Why it matters:** the row says Abort and the app rewrites a commit. That is journaled, so `u` undoes it, but it is the opposite of what he asked for at the moment he is most careful. The "You cannot lose work" promise holds only through undo. The label and the action disagree (UX: match between system and the world).
- **Fix:** run each row in the context it was spelled in. For Banner rows, call the banner's handler directly (or set `banner_focus` for the dispatch). Better: dispatch by `ActionId`, not by re-pressing a chord.
- **Tests to add:** from the Graph during an edit stop and during a conflict stop, `? ↓↓ ⏎` on Abort yields the abort preview, and Skip yields skip.

### 2. A cursor on a placeholder never moves to another group's landed row

- **Where:** `crates/graft/src/update/palette.rs:1219-1233`. `landed()` only searches `in_group` rows: the placeholder's own group.
- **The spec says otherwise:** ticket 12's Decisions and the design note (`docs/design-notes/m8-polish-and-ship.md:177`) say the cursor moves to **the first runnable row that lands**. The function's own doc (`:1210`) narrows this to "of that group".
- **Effect:**
  - `⌃P v2 ⏎` while BRANCHES still shows `reading…` leaves the cursor on `reading…`, even after a TAGS or COMMITS row has landed.
  - `⏎` on a placeholder does nothing.
  - On a large repo (switch_refs ~1–2 s at 50k refs, gap `M8/12-palette-perf`), J27 is no longer 3 steps.
- **Test gap:** `cursor_on_placeholder_moves_to_first_landed_row` covers only the same-group case. The pty walks wait for the cursor before pressing `⏎`, which hides the cross-group case.
- **Unverified at runtime:** reasoned from code; no run.
- **Fix:** if the anchor is a placeholder and its own group has nothing runnable yet, fall back to the first runnable row in any group. Add a unit test where TAGS lands while BRANCHES is still `reading…`.

### 3. `update/palette.rs` is 1462 lines (rule: ~400)

- **Where:** `crates/graft/src/update/palette.rs` (PROMPT.md 6.2).
- **What:** one file holds the listing and filter, the find jobs and their landing, running a row, keys, edit, paste and mouse.
- **Why it matters:**
  - Major 1 and Major 2 both sit in code that is hard to read in one go.
  - `empty_state.rs` already duplicates part of it (Minor 7).
- **Fix:** split into `palette/listing.rs` (groups, rows, filter), `palette/find.rs` (ask_* jobs, `landed`, cancel), `palette/run.rs` (`run`, `first_runnable`) and `palette/input.rs` (keys, edit, mouse). No behaviour change.

### 4. ARCHITECTURE.md no longer matches the code

- **Matcher placement:**
  - `crates/graft-core/src/matcher.rs` puts an algorithm in `core`, which ARCHITECTURE.md:399 and ADR-0007 say holds plain data only.
  - The justification at `matcher.rs:4-7` is false: `graft` already depends on `graft-ops` (`crates/graft/Cargo.toml:29`), so `graft-ops` could host the matcher with no new edge.
- **Loop-carried commands:** `Command::Retheme` is a seventh command carried out by the frame loop (`event_loop/running/turn.rs:303-316`; `jobs/runner.rs:1469` `not_a_job`). ARCHITECTURE.md:159-161 still says "Six commands are the exception".
- **Missing entry:** there is no "Changing the palette" entry in the where-to-change list.
- **Why it matters:** ARCHITECTURE.md is what builders read before moving modules (PROMPT.md 6.1). A false layering reason will be copied.
- **Fix:**
  - Move `matcher` to `graft-ops` (it is pure, so the move is mechanical), or write an ADR amending ADR-0007, and fix the doc comment either way.
  - Make "six" read "seven" and list Retheme.
  - Add a "Changing the palette" row.

## Minor

1. **More files over size:**
   - `crates/graft-ui/src/palette.rs`: 430 lines.
   - `crates/graft/src/empty_state.rs`: 441.
   - `crates/graft/src/session.rs`: 510.
   - Fix: split each on the seam it already has (renderer vs hit-testing; start-screen palette; theme choice).
2. **Allow with no reason:** `crates/graft-ui/src/palette.rs:350` has `#[allow(clippy::too_many_arguments)]` with no `reason`. Every other allow in the repo carries one. Fix: add the reason, or pass a small struct.
3. **Unreported ranking change in `crates/graft-ops/src/switch_list.rs` `rank`:**
   - A segment now starts after any non-alphanumeric character, where before it was only `/` or `-`. That reorders picker results for names with `_` or `.`.
   - The NOTES report only the space change.
   - Fix: record it in the ticket NOTES and add a picker-order test for `_`/`.` names.
4. **Clicks below a short box are swallowed:** `crates/graft/src/view/frame/layout.rs:291-303` tests "inside" against the **maximum** `palette_area`, not the drawn box. A click on the graph below a short box (`docs/shots/m8-palette-find-120x36.txt` ends at row 16) returns `Inside` and does nothing, instead of closing it as 11b decision 6 says. Reasoned, not run. Fix: hit-test against `box_rect`.
5. **`more` row with nothing more:** `crates/graft/src/update/palette.rs:589` shows `more` when `hits.len() >= COMMIT_HITS`. The job caps at `COMMIT_HITS`, so exactly that many hits offers "more" that leads to nothing new. Fix: ask for `COMMIT_HITS + 1` and show `COMMIT_HITS`.
6. **Listing rebuilt 3–4 times per key:** `rows()`/`flat()` run once each in `edit`, `landed` and `first_runnable`, plus once per frame (palette.rs). That is about a third of the update budget at p95 (~5 ms, from 12b/12c NOTES). Fix: compute once per update and pass it in. Overlaps gap `M8/12-palette-perf`.
7. **Duplicated logic:** `crates/graft/src/empty_state.rs` `outside_paste` repeats `first_runnable` (`update/palette.rs`). Fix: call the one function.
8. **Problem label spelling:** a theme problem is labelled `[theme.high-contrast]` even when he wrote `high_contrast` (`crates/graft/src/config/theme.rs`). Fix: echo the key he wrote.
9. **Journey numbers wrong in walk docs:** in `crates/graft/tests/it/palette_find.rs`, the header says J26–J28, and the walks say "J30 seek-commit-words" and "J31 file-history" where the note has J29 and J30. Fix: renumber to match the note.
10. **Ticket 11 not test-first, no gap row:** its NOTES admit the tests followed the code, and no KNOWN_GAPS row records it (12b and 16 have theirs). Fix: add the row, or state it in the log.
11. **Test file size:** `crates/graft/tests/it/palette_find.rs` is ~900 lines. Fix: split by journey.

## Questions

1. **Does a seek from under an overlay leave the graph visibly focused?** A branch/commit seek started while Journal, Stash or Map is open sets `detail.focus = Graph` (`update/palette.rs:1258-1261`) but does not close the overlay (`edge.rs:84-86`). The note's "Where he lands" says the graph is focused, so is that what he sees? Needs a capture.
2. **Was the find prototype skip confirmed?** The design note (`:97-98`) says find had no step-5 prototype and asks the conductor to confirm that reading. No confirmation was found in the log.
3. **Should Esc after a FILES row go back to the prior place?** A FILES row picked from inside a place calls `open_file_history`, which leaves that place's trail (`update/place/history/open.rs`, `trail::leave(state, commands, false)`). So Esc lands on the graph, not the place he came from. Intended (same as `F`?), or should it stack?

## Already gapped (not re-filed)

`M8/12-palette-perf`, `M8/12b-perf-pty-write` (broken), `M8/12c-release-run-9-vs-7`, `M8/12c-ghost-rows-all-badges`, `M8/12b-empty-history-before-status`, `M8/11-j7-handover-unobserved`, `M8/11-start-theme-lost`, `M8/11-start-help-dead-keys`, `M8/09e-start-help-n-next-match`, `M8/16-theme-colour-unverified` (rule 5), and the 11c/11d doorway rows.
