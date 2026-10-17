# Review — ease-pass group 06: graph fold by default

Scope: tickets 06 (engine + continuation 1), 06d, 06e, 06f, and the debugger's count-at-rest fix (068b98c3).
Range `a00dc75b^..89325b16`, restricted to the 06 commits: 0859e08f, 84fbccde, 7a4c11f0, 7af1cce6, 775cc69a,
dda0a592, 9c29510e, f107ab18, 068b98c3, plus the rule design e20f6a94. Mock: `prototypes/ease-mocks/mergewright-main-screen.html`.
Reviewer: read-only, at 89325b16. Skill: `code-review`.

**I did not run cargo, as I was told.** Two builders held the CPU. Every finding below comes from `git grep`/`git show` reads at 89325b16.
I re-ran no gate, test, capture or perf number. "Green" in a ticket is the builder's claim, not mine.

## Verdict

| Severity | Count |
|---|---|
| Critical | 0 |
| Major | 6 |
| Minor | 12 |

- **No git write sees a folded view.** This holds as far as reading can show.
  - `Row.commit.parents` is git's commit under every inclusion. The property test pins it: `inclusion::tests::real_parents::every_row_keeps_its_real_parents_under_every_inclusion` (`crates/graft-graph/src/inclusion/tests/real_parents.rs:14`).
  - Every reader that builds a git write takes the real parents:
    - the revert and Fix menu `-m`: `update/context_menu.rs:59`;
    - take refusal: `update/preview/take.rs:131`, `context_menu/verbs.rs:206`;
    - first parent only: `update/place.rs:416`;
    - the evil-merge probe: `update/selection.rs:208`.
  - `drawn_parents` is read only by drawing code: `ancestry.rs:94-104`, `fold.rs:229`, `raster/node.rs:32`, `update/preview.rs:456`, `view/preview.rs:168`.
  - The one `Commit` still built from drawn parents is the preview's `resident` slice (`update/preview.rs:451-457`). The plan job gets ids only (`update/preview.rs:411-414`), so no write sees that slice. Its drawing knock-on is Minor m1.
- **The count-at-rest fix fixes the cause.** Before it, the loop scheduled the rest wake only while a follow was owed (old `rest_deadline`). A count deferred by a burst therefore waited for an unrelated key.
  - Now `rest_owed = follow_behind || fold_count::owed` (`update/mod.rs:228`), and `rest_deadline` reads it (`event_loop/running/turn.rs:344-347`).
  - No sleep and no polling.
  - It cannot spin: `rested()` resets `Rest` to its default, so `deadline()` is `None` until the next motion (`event_loop/rest.rs:121-130`). Each `Rested` turns one merge's entry into `Counting` (`update/fold_count.rs:41`).
  - The red-first test is `event_loop::tests::settle::fold::a_count_answered_mid_burst_asks_the_next_at_rest` (`settle/fold.rs:238`).
  - Its cost (`owed()` scans the screen's rows on every turn) is owed to the release perf run.
- **The fold rule matches the note.**
  - Note, Hard choice 2 (`docs/design-notes/ease-pass.md:203-220`): the first-parent closure of the keep tips and of every ref tip that no shown merge reaches through a second parent; "merged" means merged into any shown line.
  - Code (`graft-graph/src/inclusion/fold_walk.rs:4-7`, `admit` :158-168): keep wins; then wanted; then forced; then pointed (hidden behind a merge); an unseen commit shows as a tip.
  - The two are equivalent: every `Pointed` chain starts at some shown merge's second parent.
  - The property oracle uses the same rule ("every commit nothing points at"): `fold_walk/tests.rs:208-235`. That answers 06g's NOTES question. Because the oracle mirrors the code, it cannot catch a rule that disagrees with the note. 06g's named dependent-branch cases are still **draft**, so the teammate cases are not pinned by name yet.

## Spot-check of ticks (rule 7 gate)

**15 checked, 14 held, 1 held in part (Major M5). No false tick.**

| Ticket | Box → test read | Held? |
|---|---|---|
| 06 | 1 `in_topological_order_the_fold_is_exact`: exact set, `behind` chain ends at a shown unopened merge, `inside`, `trim`, `folds` | held |
| 06 | 5 `fold_by_default::first_paint_folds_merged_branches_and_the_tails_name_them`: no merged commit in the window, a fold fact per row, both `Some(true)`, both tails | held |
| 06 | 6 `fold_count::tests::every_folding_merge_on_screen_is_counted_one_job_each`: jobs `[first, second]` | held |
| 06 | 7 `space_opens_the_merged_side_on_its_own_lane_joining_the_merge` (`lane ≠ merge.lane`, `outgoing ∋ (lane, tip)`, `inside`) + `graph_toggles::*` | held |
| 06 | 8 `settle::fold::a_held_j_over_a_folded_walk_follows_and_counts_only_at_rest`: asserts 61 rows, 61 facts, 60 `Some(true)`; 0 counts mid-burst, 1 at rest | held |
| 06 | 9 `capture_main_folded_and_opened_at_120`: both shots, captions name the mock | held |
| 06d | 1 `real_parents::every_row_keeps_its_real_parents_under_every_inclusion` | held |
| 06d | 3 `folded_merge::*`, "radar markers" | **in part**: see M5 |
| 06d | 5 `head_keep::refs_arriving_with_head_unchanged_keeps_the_view` (no request, phase and view equal), `switching_to_a_merged_branch_shows_its_tip` | held |
| 06e | 2 step counts: `fold_merge_count` route `["Space"]` = 1 (`m8_walk/journeys.rs:268-272`) | held |
| 06e | 3 boxes 5 and 7 (above) | held |
| 06e | 4 held-`j` (above) | held |
| 06f | 2 `graph_column::tests::a_merge_whose_side_is_not_drawn_is_still_a_merge` | held |
| 06f | 3 box 9 capture (above) | held |
| 06f | 5 `a_count_answered_mid_burst_asks_the_next_at_rest`: asserts merge 1 at the key, merge 2 at rest, no follow owed | held |

06f box 5 ("full workspace nextest + doc green") is a run claim I could not check without cargo.

## Feature pipeline

- **Need:** note :24-31. **Prior art:** note :32-37 cites `docs/research/graph-readability.md` §1 and §3. SM and Fork fold merges by default; the complaint is SM #2009, with links. **Journeys:** `open-folded-merge 1 space` (SM 1 click), note :68. **Prototype:** the accepted main mock.
- **Step count:** the build's walk counts 1 key (`fold_merge_count`), equal to the note.
  - The walk counts against m8's fence id `fold-merge-count`, not the ease-pass id `open-folded-merge`. No walk reads the ease-pass id (Minor m11).

## Critical

None.

## Major

**M1. Older stash entries now draw `◉` (merge) in the Unicode tier.**
- **Where:**
  - `crates/graft-ui/src/graph_column/kinds.rs:46-51`: 06f made `NodeKind::of` read `row.commit.parents.len()`.
  - Since 06d (D1), a stash entry's `commit.parents` holds its index and untracked parents, so 2 or 3 parents.
  - `crates/graft/src/view/entries.rs:300-308` sets `NodeKind::Stash` only for `refs.stash()`, which is the newest entry alone (`crates/graft/src/refs/mod.rs:53-55`).
  - So `stash@{1}` and older fall through to `NodeKind::of` → `Merge`. Before 06f they drew `●`, because `drawn_parents` was trimmed to one.
- **Why it matters:**
  - A stash entry reads as a merge commit. That is two different things looking the same (field rule).
  - It contradicts 06 D1's own line, "Stash rows: `is_merge()` is false for them".
  - 06f's Touches says "no other test changed". No capture has two stash entries, so nothing would catch this. **Unverified on screen**: I did not run it.
- **Fix:** derive the node from `history::parents::is_merge(row, refs)` in the app (it already excludes every stash entry), or set `NodeKind::Stash` for every `refs.is_stash(id)`. Add a test: a row for `stash@{1}` draws `■`/`●`, never `◉`.

**M2. The tiers disagree: the pixel tier still draws a folded merge as a plain commit.**
- **Where:**
  - `crates/graft-graph/src/raster/node.rs:31-37`: `Ring::of` reads `row.drawn_parents.len()`.
  - Its caller is `crates/graft/src/view/strips.rs:375-381`.
  - 06f moved only the Unicode `NodeKind::of` to the real parents.
  - Result: a folded merge (and a merge in `1`) is `◉` in Unicode and a filled disc in pixel strips. Meanwhile the overview strip marks it as a merge (`view/strip.rs:47`).
- **Why it matters:** 06f's Touches lists `NodeKind::of` callers only (`graph_column/mod.rs:121,146`). Its twin in the other tier was not checked, which is the knock-on rule. Ghostty users see a different picture from the daily Unicode view.
- **Fix:** have `Ring::of` read `commit.parents`, with the caller excluding stash entries as in M1. Alternatively, have the app pass the ring from the same `is_merge` answer. Add a raster test that mirrors `a_merge_whose_side_is_not_drawn_is_still_a_merge`.

**M3. Fold by default shipped ahead of 06c, and its interim breakage is not in `KNOWN_GAPS.md`.**
- **Where:**
  - Every route to a commit the default fold hides ends in "not in this view":
    - search `n`/`N`: `crates/graft/src/search.rs:333`;
    - detail parent links, `J` put-back and palette landings: `crates/graft/src/update/detail/landing.rs:237-239`.
  - The detail pane now lists a folded merge's second parent (06d `a_folded_merge_shows_both_parents_in_the_detail_pane`). Following it lands on "not in this view".
  - 06f saw this case and filed it as a question: `m8/search-hint-120x36`, `/topic 4 matches (1/4)` with 3 of them folded.
  - `KNOWN_GAPS.md` has no ease-pass 06 row (`grep ease/0` gives 01r–05r only). Row :278 still describes this as a choice the user made ("first-parent/folded").
- **Also unrecorded** (the 06 NOTES call these unverified):
  - `FoldWalk` ≈5.75 B/commit and `Row` +24 B, unmeasured (ADR-0026);
  - repair and rejoin cost, unmeasured on a real repo. Each sync merge emitted before its second parent re-lays out from its row (`included/redecide.rs`; NOTES "Cost of repairs"). His team merges base into feature often.
- **Why it matters:** rule 4. This is daily-use breakage on main until 06c lands, and nothing records it.
- **Fix:** add `ease/06-reveal` (**broken**, closes with 06c), `ease/06-memory` and `ease/06-repair-cost` (**unverified**, milestone review).

**M4. Help, palette and man page say `space` folds; it now opens.**
- **Where:**
  - `crates/graft-ui/src/keymap/table.rs:181-185`: `"Fold a merged branch into its merge"`;
  - `docs/man/mwr.1:165`;
  - drawn in this milestone's own sheet, `docs/shots/ease-pass/ease-pass-contact-sheet.txt:270`.
  - Merged branches open folded, and `space` opens them (`02-keymap.md:283`).
- **Why it matters:** this is a textual finding on a capture. `?` and `what_does_key`, his lookup route, describe the inverse of what the key now does by default.
  - 06f's Touches says "Keymap … none".
  - 06's Touches named `02-keymap.md §2.3 Space` as V's, but no one owned the label.
- **Fix:** relabel to "Open or close a merged branch" (or the note's words). Regenerate the man page and the help captures. Update the §2.3 row at `02-keymap.md:276`.

**M5. 06d box 3 "radar markers" is held in part: the graph's radar marker is untested on a folded merge.**
- **Where:**
  - The box names five readers. Its "radar markers" test, `update/tests/folded_merge.rs:106-121`, asserts the **detail pane's** radar line (`view::detail_view` → `view/detail.rs:137-142`).
  - The graph row's marker, `view/entries.rs:341-346` (`marker_for`), is the reader 06d's own spec listed as "radar markers" (`view/entries.rs:340`). That marker and the strip marks (`view/strip.rs:47-58`) were changed and have no folded-merge test.
- **Why not Critical:** the test does assert what its words say ("the radar treats it as a merge"), and all three readers take the same `history::parents::of`. The tick overstates; it is not false.
- **Fix:** add `a_folded_merge_row_draws_its_radar_marker`. Build the window from `folded_merge_on_screen`, put the evil-merge finding in, then assert `visible_rows` gives the merge row `marker == Some(..)` and `strip_marks` holds `Radar` at its row.

**M6. Production files already over 400 lines grew, and the ratchet was raised instead of splitting (PROMPT 6.2).**
- **Where:** `crates/graft/tests/it/over_400.txt` rows raised by 06 commits:
  - `update/keys.rs` 483→487;
  - `update/selection.rs` 636→639;
  - `update/state.rs` 1307→1312;
  - `update/preview.rs` 464→470;
  - `view/detail.rs` 403→407;
  - `view/entries.rs` 429→435 (net down from 446);
  - `view/radar.rs` 731→732;
  - `view/rows.rs` 893→894.
- **Why it matters:** the ratchet exists to stop growth. Each step is small, but together they add up to 8 production files growing in one group.
- **Fix:**
  - Move the new code out: `apply_view_change`'s landing into `keys/view_change.rs`; `resident_slice` into `update/preview/slice.rs`; `AppState.facts_pending` and `fold_counts` docs into their modules.
  - Lower the rows back down.

## Minor

**m1. The ghost preview draws a folded merge as `●`.**
- `update/preview.rs:451-457` builds the resident `Commit`s with `parents: drawn_parents`, and `update/preview/slice.rs:139-146` clips them further.
- `NodeKind::of` (now real-parent based, `kinds.rs:47`) therefore sees one parent for a folded merge inside a preview.
- The preview's `Row.commit` is not git's commit, which breaks D1's invariant inside the preview. Today it is drawing-only.
- **Fix:** carry the drawn parents beside the commit in the slice, or set the node from the source row.

**m2. The fold tail disappears whole when it does not fit.**
- `graft-ui/src/graph_screen/render.rs:655-658` returns `None` unless the whole tail plus one cell fits.
- At 80 columns a long name (`▸ 6 commits from omar/eligibility-api`) drops the only fold cue, and the merge looks plain.
- This is the same family as KNOWN_GAPS M5/08 p6 (narrow space drops the informative part first).
- **Fix:** fall back to `▸ N commits` (drop ` from …`), then to `▸`.

**m3. The tail gap differs from the mock.**
- `render.rs:642`: `FOLD_GAP = 1`.
- The mock puts two spaces before the tail (`mergewright-main-screen.html:188`, `<span class="fold">  ▸ …`). Build to the mocks, or say why not.

**m4. Question: a refresh whose selected commit is now folded keeps the cursor's index, not its commit.**
- `history/scroll/apply.rs:44-51`: the reanchor is not found, so the cursor keeps its index.
- The landing on `behind` is wired for promotion only (`history/serve.rs:263-268`, `request.promote.as_ref()?`).
- Case: his cursor is on a teammate's branch row; a fetch or `Ctrl+R` brings that branch's merge; the cursor silently sits on another commit.
- 06c may own this ("any route that goes to it").

**m5. Question: a window re-sent after a repair can move the selection.**
- `history/views/sent.rs:111-131` re-sends with `promoted: false`, so the screen takes it as a plain window (`apply.rs:36-52`) with no reanchor.
- A late commit forced shown above the cursor shifts which commit the cursor's index names.
- Date order only (no commit-graph), so rare. Unverified.

**m6. Question for 07: the cursor `▸` and the fold tail `▸` share a glyph.**
- Both appear on one row (`▸ ◆ … ▸ 1 commit from sara/fix-toc`, `main-folded-120.txt:6`).
- Tests now tell them apart by column (06e).
- This is different things looking the same.

**m7. Two answers to "is this a merge".**
- `Row::is_merge` (`graft-graph/src/row.rs:202`) has no production caller; the app uses `history::parents::is_merge`.
- D1 named `Row::is_merge` as "the one answer".
- **Fix:** delete it, or make its doc say "drawing-free, stash-blind; the app uses `history::parents`".

**m8. Stale words:**
- `TESTING.md:3333` (J33 still `+` / `+N`, "fold a merge with Space");
- `KNOWN_GAPS.md:156` ("a folded merge looks like a plain commit — only the header's N merges folded", which was retired);
- `jobs/fold_count.rs:1` and `jobs/runner/lists.rs:169` (`+N`);
- `graft-ui/src/overview_strip.rs:22` (06d noted it; no row);
- `ARCHITECTURE.md:486-490`, which says nothing about the `Row.commit` / `drawn_parents` contract or fold by default (PROMPT 6.1);
- `docs/design/03-graph-layout-algorithm.md` §3.6 (no default, no late repair).

**m9. Ticket bookkeeping:**
- `06f-fold-captures.md:3` says `Status: ready` though every box is ticked;
- `06d-…:3` says partial, with box 4 unticked; 06f closed it (`take-refused-merge` re-shot);
- 06e's last box is unticked; 06f closed it.
- The conductor should close them, citing 06f.

**m10. 06e skipped the cause it was asked for.**
- Its spec asked for a cause with a line for `m7_walk::pr_contents`, `preview::the_branch_badge…` and `radar_not_checked`.
- NOTES says "already passed after 06d + main; not diagnosed".
- They were 120 s timeouts in continuation 1, and the likely cause is D1. The cause is unrecorded.

**m11. No walk reads the ease-pass `open-folded-merge` fence.**
- `fold_merge_count` counts against m8's fence `fold-merge-count` (`m8_walk/journeys.rs:272`), which has the same count, 1.
- Ticket 14's walk must read the ease-pass fence id.

**m12. The bar never offers `space` on a folded merge.**
- `RowBar::FoldedMerge` / `OpenedMerge` (`graft-ui/src/graph_screen/hints/lists/rows.rs:115-123`, `space show the N commits inside`) have no producer in `crates/graft/src` (grep empty).
- 06 put "the bar per row (G)" out of scope. Recorded so 07 owns it: the step after a fold is not visible yet.

## Knock-ons checked

| Knock-on | How checked | Result |
|---|---|---|
| Undo/journal | `update/undo/answers.rs` diff (05's picks only); `ViewChange::Fold` builds no `Command` beyond `ShowRows` | a fold is not an operation; nothing journaled. OK |
| Preview | `resident_slice` → ids to the job (`preview.rs:411-414`); ghost rows built from the plan, not the fold | no write sees the fold; drawing glyph m1 |
| Search | `search.rs:310-334` | folded hits counted, `n` → "not in this view": M3 |
| Pixel strips | `strips.rs:375-381`, `strip.rs:44-58` | ring disagrees with Unicode (M2); strip marks real-parent (OK) |
| Held-`j` settle | `turn.rs:319-347`, `update/mod.rs:233-262`, `settle/fold.rs` | the count waits for rest; the rest wake is owed for counts; no spin |
| HEAD keep (D2) | `tree_answers.rs:16-62,108-125,267-288`; every walk goes through `refresh` with `OpenRepository` (`keys/refresh.rs:51-53`, `update/mod.rs:120-131`) | one layout per launch; holds |
| Stash rows | `history/parents.rs`, `entries.rs:300-308` | semantic readers OK; Unicode glyph M1 |
| Escapes | `view/rows/fold.rs:76-77` `sanitize_line` on the recovered name; `Inside.label` raw but dead (`expect(dead_code)`, 07) | OK; 07 must sanitise `label` |
| `unwrap`/`expect` / swallowed errors in production diff | grep of each 06 commit's non-test `src` diff | none |

## Owed to the milestone review (not run here: no cargo)

- Gates: fmt, clippy `-D warnings`, nextest, `test --doc`, and clippy `--features heavy-tests`.
- Heavy tier, `--test-threads=2`.
- **Release perf run** (`frames::`, `heavy::budgets_hold`). Specifically:
  - first paint now waits for the repository opening before the first row request (`history.head_pending`, `tree_answers.rs:114-116`; `scroll/apply.rs:186-190`);
  - the cost of `lay_out`'s repair loop (`history/fold.rs:58-84`) on a repo with many sync merges and no commit-graph;
  - `rest_owed` evaluated every loop turn.
- Memory run (ADR-0026): `FoldWalk` ~5.75 B/commit plus sparse maps; `Row` +24 B and a heap `Vec` per resident row; `Window.fold`.
