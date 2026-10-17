# Review — ease-pass 03 + 03b (ease-vocabulary)

Read-only against `340027a6`. Diffs: `2b75726d^1..2b75726d` (03, partial 5/10) and `340027a6^1..340027a6` (03b).
Tickets: `.scratch/ease-pass/issues/03-ease-vocabulary.md`, `03b-ease-vocabulary-rest.md`; scout `scouts/03-ease-vocabulary.md`.

**Counts: 0 Critical · 3 Major · 11 Minor.**

## Gates: not run by me

The conductor said **don't run cargo**, because two builds were using the CPU. So I ran nothing: no fmt, clippy (plain or `heavy-tests`), nextest, heavy tier or release perf run. I have no fast-suite time, slowest tests, peak memory or `target/` size. The conductor's log says gates passed on main at `340027a6` (fmt ok, clippy 0, nextest 6560/6560, doc ok; `docs/log/ease-pass.md:48`). I did not check this. **The heavy tier and release perf run are still owed for this group.** `Hint` lost `Copy`, and named verbs now allocate on every `hints_for` call. That cost is almost certainly tiny, but no frame budget has measured it.

## Ticks spot-checked: 12 checked, 12 held, 1 not checkable

- **03 (5 ticks):** :64 `the_review_look_keys_are_still_toggle_diff`, :65 `switch_and_compare_keep_their_keys` and :67 `no_new_context` all exist and assert what the ticks say. :69: the contrast test loops every theme, and the dark values match the mock's `--this/--under/--trunk/--other` (`mergewright-main-screen.html`, CSS vars). :71: I counted the `02-keymap.md` diff myself: 6 hunks and 7 `Proposed (ADR-0063)` marks. All held.
- **03b (8 ticks):** :72: no `.snap` file appears in the diff. :73–:76: each named test exists and asserts the quoted words. :77: the sweep arms exist, and `classified_form → None` is disclosed. :78: I read all five `editor_tests`. All held. :79 (gates) is not checkable, because I ran no cargo.
- **Weakness, not a false tick:** :73 claims "03 Handoff's words exactly" and holds against the Handoff. But the Handoff's words for `Opened` do not match the mock (Major 1).

## Deviations: both accepted

- **`BannerPaused`.** The reason is sound. `Banner { merge: true, conflicted: true }` is also the first landing, where "back to" would be false, and reusing it would have changed 8 committed captures. The ticket's fallback condition was a slightly different one ("`⏎` does not reach the resolver"), but the builder's reason is stronger. **Nothing picks it yet, and no downstream spec knows it exists** (Major 2).
- **`ClashOp::Other(SideRoles)`.** The reason is verified: `lists/clash.rs:116-125` holds `take_words` equal to `SideRoles::ours/theirs` for all six roles, so a rebase says `take upstream (left)`, never `yours` for the wrong side. It does leave one state that can be built but means nothing (Minor 3).
- **`Hint.verb: Cow<'static, str>`.** It is clean: table words stay `Cow::Borrowed` and const-constructible, and the fitter measures the one string it draws. The added `named_short` field keeps "a short rung is a prefix of the verb", and `every_short_rung_is_a_prefix_of_its_verb` checks it over `every_hint`. The `.verb` readers in graft are test-only (`spelled.rs` is `#[cfg(test)]`, `lib.rs:29`), so `Spelled::said`'s `Box::leak` never ships.

## 02-keymap edits

All 7 marks say `Proposed (ADR-0063)`, and they match the mocks and the "all a" answers:
- §2.3 `#`: Q3 a
- the fold paragraph: Q1 a and Q2 a kept
- §2.3b: Q6 a, scoped by preview kind
- §2.6 table: Q7 a and Q12 a; `y`, `n`, `u` and `esc` as the merge mock's handler at :150
- §2.6a
- §2.6b: as the mock at :159; `Space` stays a `finish.toggle_diff` alias, and the mock gives `space` no meaning
- §2.10: C11, C15, Q8 a

One drift: §2.10 still says `?` lists the options while the screen is open, but 03b decided that `?` types in the subject and body (Minor 8).

## Key consistency (ux-principles §2 h3/h4/h6/h10)

`u` means "undo the last thing" on every bar (`undo`, `undo the commit…`, `undo the merge`, `undo pick`), which is consistent. `c` means commit on the graph and finish/continue at a stop, as today. Problems are Minors 1, 2, 9 and 10.

---

## Critical

None.

## Major

### 1. `RowBar::Opened` draws words neither mock row has
- **Where:** `crates/graft-ui/src/graph_screen/hints/lists/rows.rs:120` draws `space hide the commits inside`, followed only by the `u undo` tail.
- **What the mock does:** `prototypes/ease-mocks/mergewright-main-screen.html:206`. On the **merge row** (`fr.fold`), opened, the mock's bar is `space hide the commits inside · ⏎ open · x compare`. On the **inner rows** (no `fold`) it is the teammate bar, `⏎ open · C switch to <b> · x compare`.
- **Where our two docs disagree:** the variant's doc comment calls it "A merge opened by `space`" (`hints/context/ease.rs:29`). But G's spec picks `Opened` for "a row with an `InsideMerge`", i.e. the inner rows (`.scratch/ease-pass/issues/07-graph-rows.md:34`).
- **Result:** read either way, the user loses `⏎ open` and `x compare` on that row. And if G follows its spec, an opened merge row has no state that says `hide`, because `FoldedMerge` says `show`.
- **Why it matters:** "Build to the mocks" (ADR-0063) — what he accepted is what he gets. The bad words came in through 03's Handoff transcription (`03-ease-vocabulary.md:82`) and the tick held against that, so this is not a false tick.
- **Fix:**
  - `FoldedMerge { inside, open: bool }`, or a separate `OpenedMerge`, drawing `space hide the commits inside · ⏎ open · x compare` for the merge row.
  - Inner rows take `Own` or `Elsewhere` as the mock does.
  - Decide whether `space` (which closes the merge) is drawn on inner rows. The mock's handler answers it (:219) but its bar does not draw it.
  - Fix 07:34 to match.

### 2. Both deviations stop at 03b: downstream specs still name the old shapes, and 05 is already dispatched
- **05b:6** says "From 03b: `Banner { merge: true }` (`⏎ back to the clashes`)". The words actually live on `HintContext::BannerPaused` (`hints/lists.rs:327-335`), and nothing picks it (`update/tests/dead_keys.rs` → `None`). `grep -rl BannerPaused .scratch/` finds only 03b. **Risk:** P re-words `Banner` (8 captures break), or never picks `BannerPaused`, so Q7 a's words never show.
- **05b:28** scopes "`graph_screen/hints/lists/resolver.rs` (`ClashOp::Merge` + `ClashAt::AllSolved` arm only)". That arm is now in `hints/lists/clash.rs:33-37`, plus `finish_word` at :99-104.
- **11:6** still says `ClashOp::{Merge, Other}`; the code has `Other(SideRoles)` (`context/ease.rs:77`).
- **11:30** says `next` is the "full path — 03b shortens nothing for files". The merge mock draws `next file: ${f.split("/").pop()}` (`mergewright-merge-screen.html:150`), i.e. the last segment.
- **Why it matters:** CLAUDE.md "Things are connected" — a deviation without its knock-on edits is the M6 failure shape.
- **Fix (conductor):**
  - 05b:6 → `BannerPaused`, picked by 05b when `esc` pauses.
  - 05b:28 → `lists/clash.rs`.
  - 11:6 → `Other(SideRoles)`.
  - 11:30 → last path segment, as the mock (or a `bar_name`-style helper for files).
  - Tell the 05 builder now.

### 3. Readability (PROMPT 6.2): modules named after the milestone, and the size ratchet raised on four source files
- **Names.** `hints/context/ease.rs` (holds `RowBar` and `ClashBar`), `hints/catalogue/ease.rs`, `hints/tests/ease.rs` and `keymap/tests/ease.rs` are named after a pass codename, not after what they hold. In two milestones "ease" says nothing.
- **Sizes.** `crates/graft/tests/it/over_400.txt` raised four source files:
  - `hints/context.rs` 511 → 535
  - `theme/mod.rs` 410 → 431
  - `keymap/table.rs` 513 → 533
  - `update/palette/mod.rs` 406 → 408

  `table.rs` and the `Token` enum grow by data rows, which is defensible. `context.rs` now holds the `HintContext` enum plus `RowVerbs`, `LiveC`, `TabWord` and `WayToBanner`, and can shed them.
- **Fix:**
  - Rename to `context/bars.rs` (or `row_bar.rs` + `clash_bar.rs`), `catalogue/bars.rs`, `tests/bars.rs`.
  - Move `LiveC`, `TabWord` and `WayToBanner` out of `context.rs` so it drops back under the cap.

## Minor

1. **Same key, two words, on one screen.** Below `MIN_COLUMNS` (60) the clash bar says `esc close` (`graph_screen/hints.rs:131-136`); at 60 columns and wider it says `esc pause, come back later` (`lists/clash.rs:40`). The module doc also claims "the three most relevant still fit a 40-column bar" (`lists/clash.rs:10-12`), which is false: at 40 the bar is `esc close`. Fix: spell it `esc pause` at the floor, and drop the 40-column sentence.
2. **Clash screen discoverability (question for L / the usability walk).**
   - The clash bar is modal with no doorway (`filters.rs:38`), and the resolver does not answer `?`. Its lists draw only the mock's keys (`lists/clash.rs:47-58`).
   - `O/T`, `⌃O/⌃T`, `N`, `M`, `B`, `d` and `j/k` still act (`lists/resolver.rs:76-101` lists them today), but once L switches to `Clash` they are shown nowhere.
   - `Picked` draws no `esc` at all (`lists/clash.rs:39`): a modal with no visible exit (ux-principles §3.2, h3, h6, h10).
   - The words are mock-exact, so this is filed as a question, not a defect.
3. **A state that can be built but means nothing.** `ClashOp::Merge` and `ClashOp::Other(SideRoles::Merge)` can both be built (`context/ease.rs:69-78`). The second draws a merge's sides with `⏎ done · c continue`. Fix: `ClashBar { at, roles: SideRoles }` with merge-ness read from `roles`, or give `Other` a non-merge roles type.
4. **The paused banner swaps its first hint by position.** `paused_banner_list` does `list[0] = …` (`hints/lists.rs:333`), relying on `banner_list` inserting `OPEN` at 0 (:315). Fix: replace the hint that `is(OPEN)`.
5. **The palette's option rows are toggles labelled as actions.** "Amend the last commit", "Skip the commit hooks" and "Allow an empty commit" (`keymap/table.rs`, the three `commit.toggle_*` rows) each flip (`update/commit_editor/keys.rs:110-122`). Chosen while already on, "Amend the last commit" turns amend **off**. The man page repeats the action wording. Two smaller points in the same code:
   - `toggle_from_palette` duplicates the id → flag map beside `CommitEditor::option_mut` (`update/commit_editor.rs:252`).
   - `opens_the_palette` calls `opener` twice (`keys.rs:94,101`).

   Fix: "Amend: on / off" wording or state in the label; map id → `Field` → `option_mut`.
6. **`debug_assert!(toggled)` vanishes in release** (`update/palette/run.rs:53`). A keyless row that is not an editor option would silently do nothing. It is commented and unreachable today. A test pinning "every keyless default is a `commit.toggle_*`" would make that hold instead of being asserted only in debug.
7. **Unverified behaviour that is not in KNOWN_GAPS (rule 4).**
   - With the palette open over the commit editor, EVERYWHERE's branch, commit and file groups and the theme/tour rows still list. A branch row seeks the graph behind the editor without closing it (`palette/run.rs:63-66`). A file row opens a history place under it.
   - At 16 colours, dark `role-under` and `role-trunk` both resolve to index 7 (`theme/snapshots/graft_ui__theme__tests__resolved_values.snap`). This is logged only in `docs/log/ease-pass.md`.

   Add both as **unverified** rows.
8. **Doc drift on `?`.** The `keymap/table.rs` comment above `commit.toggle_*` ("`?` and `⌃P` list them while it is open") and §2.10 Proposed (`docs/design/02-keymap.md:536-538`) both say `?` lists the options. But `?` types in the subject and body (`commit_editor/keys.rs:84-96`; design note :137-138). Fix: "from `⌃P`, or `?` off the text fields".
9. **`y` meaning (usability-walk flag, pre-existing).** `y` copies the SHA on the graph (`02-keymap.md:279`), but means "go on" in the preview, clash screen, review and look (`:298, :367`, §2.6b Proposed). The merge journey `m y o y y` ends on the graph, so one extra `y` silently copies a SHA. Harmless, but it is the one place `y` does not mean "go on".
10. **Interim false word between G and B.** G (07) will draw `C switch to <b>` (`lists/rows.rs:110`). Q1 a's "list opens on that branch" is B's `open_on` (09), and 09 is blocked by 07. In between, `C` opens the picker somewhere other than that branch. Fix: land `open_on` before or with 07, or have G draw `C switch` without the name until B.
11. **Weaker tests and an unchecked override floor.**
    - `each_row_bar_leads_with_the_mock_words_at_120_columns` checks only the lead and `? keys` (`hints/tests/ease.rs:58`). The middle of the bar (the `u undo` tail, `. actions`) is unpinned, and it is what Q11 a keeps.
    - The role floor (3.0:1) exists only as a test on the built-in themes. `contrast_misses` (`theme/overrides.rs:89`, `CONTRAST_RULES`) has no role rule, so a user's `role-this` override at 1.1:1 gets no warning.

    The order of the teammate row (`m merge` first, `lists/rows.rs:106-113`) follows the ticket ("its order as that mock shows it"). The design note :136-137 says the opposite ("then the merge mock's `m`"). Question for the usability walk.

## Checked, no finding
- No `unwrap`/`expect` outside tests. `bar_name`'s `unwrap_or` cannot fire, because `rsplit` always yields at least once.
- Names in bars are typed `SanitizedText` (`RowBar::Elsewhere`, `ClashAt::FileSolved`).
- No git work, no I/O and no new crate dependency (graft-ui → graft-term already existed).
- The man page was regenerated (03: three `(none)` rows plus four tokens; 03b: no id change).
- The palette's "only `CommitEditor` rows from the editor" holds in both listing sites (`groups.rs:43,151`). From the graph, no `CommitEditor` row is listed (`from_the_graph_no_commit_editor_row_is_listed`).
- Feature pipeline: covered by the milestone design note (Need, Prior art, Journeys). This group adds vocabulary only; it has no journey of its own to count.
