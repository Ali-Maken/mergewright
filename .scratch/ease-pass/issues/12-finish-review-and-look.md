# finish-review-and-look — the review as the mock ("Nothing committed yet", read-only message), landing on the folded merge

Status: done (8 of 11 boxes ticked; 3 open with reasons, see Acceptance and NOTES). Was: ready (reconciled 2026-10-18 with scout `.scratch/ease-pass/scouts/12-finish-review-and-look.md` at 3b734080: 9 mismatches folded in; split — the read-only look → 12b)
Blocked by: none open (05b, 05s door, 06 fold counts, 08 `landed_bar` landed per scouts). If `view/rows/context.rs::cursor_row_context` is missing, stop: `blocked: needs 07`.
Build with: Opus (builder-opus) — takes the text fields out of a live state machine, changes the commit's message source, and gates the landing bar on the fold count; knock-ons are wide
Needs: graft-ops `merge_review`/`MergeReview`, `FinishSummary` (`message`, `by_git`), `FileState` (`finish_preview` is `pub(crate)` in **graft-ops** — never called); 05 `Picks` (per-file picks); 05b the review's `y`/`u`/`esc` handlers; 05s `review_door`, `summary_landed` (`update/finish/answers.rs:19-70`); 06 `FoldCount`, the count read; 03b/03r `HintContext`, `RowBar::JustMerged { inside }`, `Cow` verbs, `hints/context/` one-file-per-state; 08 `landed_bar` in `view/rows/context.rs::cursor_row_context`.

## Decided here (scout mismatches)
1. **Crates (scout 1):** `merge_review` and `finish_preview` are graft-ops (`finish/review.rs:37,97`); the app reads only `merge_review` (`jobs/finish.rs:25`). No graft-ops edit in this ticket.
2. **Keys by id (scout 2):** `files_key` (`update/finish/keys.rs:95-111`) matches raw `KeyCode`: `y` :102, `u` (`asks_undo`) :103, `⏎`/`space` → `toggle_diff` :104 (handler :237), `j/k/PgUp/PgDn` :105-108; mouse toggle `finish/mouse.rs:74`. Match them by keymap id (`confirm.yes`, `app.undo`, `finish.toggle_diff`, Global move rows) — default behaviour unchanged, a remap now follows (the bar already spells them from the keymap).
3. **Frame = mock :155-162 (scouts 3, 4, 6, 8)**, copied exactly, names swapped: title `All N clashes solved. Nothing committed yet.`; `This merge will put into <into>`; `N commits from <from>`; one row per clashed file with how it was solved (`you chose: ◀ yours`, mock :158); one row `✓ N files git combined on its own` (mock :160 — `Combined by git (N)` is the clash screen's words, :147); `Message  <subject>`, read-only (:161). No undo line in the mock → `undo_line` leaves the review (Minor 7).
4. **How-solved words beyond the mock** (mock gap; ours, NOTES): one kind → `you chose: ◀ yours` / `▶ theirs` / `◆ both`; several → `you chose: ◀ yours (1) · ▶ theirs (1)` (◀ ▶ ◆ order, zero kinds left out); a whole-file card choice → `you chose: <the card's word>`; no picks recorded (hand edit, relaunch) → `solved in the file`. `N clashes` in the title = picks + 1 per whole-file or `solved in the file` file.
5. **Message read-only (scout 8; design note :157-158 gap rule):** the subject/body fields, `FinishFocus` (`update/finish.rs:55`), `draw_message`'s editors (`finish_summary.rs:223`), `from_draft`, the non-empty-subject check (`keys.rs:279`) and the text-field half of `finish/keys.rs` (subject `:161`, body `:187`, `escape` `:339` field arms, `paste` `:377` — confirm the extent) leave. The merge commits with the read's `message` (`MERGE_MSG` minus comments), as `git commit --no-edit` would. So **no keys.rs split**: the file shrinks (NOTES: lines before/after). `Finish`-context keymap rows that only served the fields are deleted (named exception to "no key table rows"; NOTES lists them). §2.6b edit text in NOTES for the conductor.
6. **Count line:** `N commits from <from>`, N = `HEAD..MERGE_HEAD`, asked in `jobs::finish::summary` beside `merge_review` with the count read 06's fold uses (name it); failed → `? commits from <from>` (06's rule).
7. **Pending (scout 9):** `open_reading` still builds `Loading::Pending` (`update/finish.rs:227-232`). State test: after `y` on the clash screen, before the read lands, the clash screen is what draws. If a review frame draws, fix the cause here: build the review state in `summary_landed`, not before.
8. **Non-merge (scout: no non-merge opener):** `HintContext::Finish { .. }` has no picker after this → listed for retirement in NOTES (hints are 03b's). `undo_line` (`frame.rs:105`, drawn :181): no caller left → removed with its own tests, each named.
9. **Landing (scout 7):** today `merged {from} into {into}` (`update/banner/resume/answers.rs:262-277`, from `finish/answers.rs:196`); cursor `finished_lands_on_head` (`banner/stop.rs:169`). Becomes `✓ Merged <from> into <into>.` (mock :197), drawn as a done status the way 08's commit landing draws its `✓` (one `✓`, never two; 08s's done line above the bar). The mock's `u undo · P push` on :197 is the bar's (03b words), not the status's.
10. **`⏎`/`space` until 12b:** still today's inline diff on a clashed row (the key acts; the bar's `⏎ look at it` becomes exact in 12b); on the combined row, nothing until 12b.

## Read first
- `docs/design-notes/ease-pass.md` M15–M19 (:148-151), mock gaps (:153-158), New UI "Review and look" (:274-275), Hard-to-reverse 3.
- `docs/design/02-keymap.md` §2.6b. `docs/design/01-view-mockups.md` §1.7 (frame only).
- `crates/graft-ui/MAP.md` `finish_summary` (:343-), `graph_screen::hints`. `crates/graft/MAP.md` `update::finish`, `view::finish`, `jobs::finish` (:132), `update::undo`. `crates/graft-ops/MAP.md` `finish` (:97-103).
- `.scratch/ease-pass/scouts/12-finish-review-and-look.md` (all). `03-ease-vocabulary.md` Handoff (review words); `03r-vocabulary-fixes.md` Decided; `05s-merge-screens-fixes.md` Decided; `08-commit-screen.md` NOTES (`landed_bar`).
- Merge mock `prototypes/ease-mocks/mergewright-merge-screen.html` :155-162 (review), :197 (landing).

## Scope
In: graft-ui `finish_summary.rs`, `finish_summary/**`; hints — `hints/context.rs` (variant line), new `hints/context/review_bar.rs`, `hints/lists.rs` (split, then the `Review` arm), `hints/lists/clash.rs` (receives the moved arms), `hints/catalogue/bars.rs`, `hints/tests/bars.rs` — `ReviewBar::List` only; graft `view/finish.rs`, `update/finish.rs`, `update/finish/**`, `jobs/finish.rs` (count only), `view/rows/context.rs` (the `JustMerged` branch only), `update/banner/resume/answers.rs` (the status words only), the keymap `Finish` rows in decision 5; sweep arms **appended** at the end of each match in `update/tests/sweep_screens.rs` and `dead_keys.rs` (11 owns those files this wave; NOTES names the lines).
- **Split first, own commit:** graft-ui `hints/lists.rs` (400): its remaining `Clash` arms → 03r's `hints/lists/clash.rs`. ≤400 after.
- **Bar:** `HintContext::Review(ReviewBar::List)` exactly `y commit the merge · j k move · ⏎ look at it · u undo last pick · esc back to the clashes (y brings you here again)` (03 Handoff), keys spelled from the keymap, drawn where today's `Finish` words go — the frame's bottom border (`footer_hints`, `frame.rs:58`, drawn :145-156); picked in `view/finish.rs` (:181-187 today). One arm in `every_context`, `screen_for`, `classified_form`.
- **M16:** `esc` → clashes, `y` → here again (05b, 05s; drawn states only).
- **M18 landing:** cursor on the merge commit, folded `▸ N commits from <from>` (06); status decision 9; bar `RowBar::JustMerged { inside: N }` picked at `cursor_row_context` beside `landed_bar` while the cursor rests on the merge just made **and** its count is `Known`; before then G's bar for the row. NOTES: how long until `Known` on the fixture. **Fast-forward** (no merge commit): never `JustMerged`, no `▸`; lands as today — NOTES records its status words (scout couldn't confirm).
- **M19:** `u` after landing undoes the merge (today's undo).
Out: the look, `ReviewBar::Look`, `look`/count reads for it (12b); `y`/`u`/`esc` behaviour (05b), the door (05s); the clash screen (11, 11b); fold (06); other hint arms.

## Signatures
graft-ui `finish_summary`:
```rust
pub struct PickCounts { pub yours: u32, pub theirs: u32, pub both: u32 }
pub enum HowSolved { Picks(PickCounts), Whole(SanitizedText /* the card's word */), ByHand }
pub struct ReviewFile { pub path: SanitizedText, pub how: HowSolved }
pub enum ReviewRow { Clashed(usize), Combined }   // what the cursor can be on
pub struct ReviewView {
    pub from: SanitizedText, pub into: SanitizedText, pub clashes: u32, pub incoming: Option<u32>, // None → `?`
    pub clashed: Vec<ReviewFile>, pub combined: u32, pub message: SanitizedText,
    pub cursor: usize, pub state: SummaryState /* existing */ }
```
graft-ui `graph_screen::hints` (`ReviewBar` in `hints/context/review_bar.rs`, re-exported): `pub enum HintContext { /* … */ Review(ReviewBar) }`, `pub enum ReviewBar { List }` (12b adds `Look`).

## Touches
`FinishView`/`FinishScreen` literals and their tests (`finish_summary/tests.rs`, 556); `footer_hints`; `undo_line` callers (grep); `preview_title` callers; `finish/keys.rs`, `finish/mouse.rs:74`; `commit` (`keys.rs:262`) message source; `update::undo` after a merge; `cursor_row_context` (08's `landed_bar` arm unchanged — its tests green unedited); the status line and 08s's done-line rule; `HintContext` exhaustive matches (graft-ui, graft); `tests/no_literal_keys.rs`; hint snapshots (existing unedited); `update/finish/tests.rs` (`over_400.txt:165` — lower its row if field tests leave; edit only own rows); captures `docs/shots/m6/fix-y-opens-the-review-187x50.txt`, `fix-finish-from-the-resolver-120x40.txt` (recaptured); `02-keymap.md` §2.6b (edit text in NOTES); `MAP.md` rows.

## Acceptance (each tick names its test)
- [x] Split first: `hints/lists.rs` ≤400, no row raised — `file_size::every_file_over_400_lines_is_listed_with_its_count` (split in commit ce9c6bd6).
- [ ] Fixture merge `sara/fix-toc` into `wallet-release`: `report.md` (2 clashes, `o` then `t`), `notes.md` (1, `o`), 2 combined; `y` → review at 120 reads, in order: `All 3 clashes solved. Nothing committed yet.`, `This merge will put into wallet-release`, `N commits from sara/fix-toc` (fixture's N), `report.md` `you chose: ◀ yours (1) · ▶ theirs (1)`, `notes.md` `you chose: ◀ yours`, `✓ 2 files git combined on its own`, `Message  <MERGE_MSG subject>` — new render test; `git ls-files -u` still lists both — same test.
  - **Open:** `finish::landing::the_review_reads_the_mock_lines_and_stages_nothing` asserts every line in order, and that nothing is staged. The one exception: the two file rows draw in git's file order (`notes.md`, then `report.md`), the read's order. The box lists `report.md` first, and the ticket gives no sort rule. Question: sort the rows, or keep git's order?
- [x] Read-only message: typing `x` and `⇥` leave state unchanged (state test) — `update::finish::tests::typing_and_tab_leave_the_review_unchanged` (also `⇧⇥`, `⌃E`, `⌃⏎`); `y` commits with `git log -1 --format=%B` equal to `MERGE_MSG` minus comments — it test `finish::y_commits_gits_message_minus_its_comments`.
- [ ] Bar at 120 exactly the List words (narrower: whole hints dropped from the end) — hint snapshot (new).
  - **Open, half holds.** The 120 words are exact: `graph_screen::hints::tests::bars::the_review_bar_draws_the_mock_words`. Narrower, each hint first steps down to a one-word rung (`commit`, `look`, `undo`, `back`) instead of dropping whole. Reason: with no short rungs, `the_floor_fits_at_the_narrowest_bar` failed. At 47 cells the floor of three did not fit, so the bar drew **nothing**, a live defect. Now 58 reads `y commit the merge · j/k move · ⏎ look · u undo last pick` and 80 ends `· esc back`.
- [x] Remap `confirm.yes` → `Y` in a test keymap: `Y` commits, `y` does nothing — `update::finish::tests::a_remapped_confirm_commits_and_the_old_key_does_not`; `update::tests::keymap_edge::sweep::every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `update::tests::bar_keys::every_key_the_bar_draws_is_answered` (`screen_for`) and `update::tests::dead_keys::every_swept_state_is_the_context_it_claims` (`classified_form`) green with `Review(List)`.
- [x] No `u takes this back` on the review (render test) — `finish::captures::summary_captures` (asserted at 187×50 and 80×24); `u` still undoes the last pick — `update::resolver::tests::review_at_y::u_in_the_review_goes_back_to_the_resolver_and_undoes_the_last_pick`. Its body is unedited; its shared fixture `reviewed()` gained `incoming: None`.
- [x] No review frame before the read lands — `update::finish::tests::landing::no_review_frame_before_the_read_lands` (decision 7).
- [ ] Landing: cursor on the merge commit; status row `✓ Merged sara/fix-toc into wallet-release.` (one `✓`); before the count lands the row's bar is G's; once `Known`, `u undo the merge · P push · space show the N commits inside` and row `▸ N commits from sara/fix-toc` — state tests.
  - **Open, one part.** These hold:
    - status and one `✓`: `update::finish::tests::landing::y_lands_with_the_merged_words_and_one_tick` and it `finish::landing::y_lands_on_the_merge_and_u_takes_it_back`
    - G's bar until `Known` (also off the merge, and on `Failed`): `update::finish::tests::landing::just_merged_waits_for_the_count_and_the_cursor`
    - `▸ 2 commits from sara/fix-toc` and the bar leading `u undo the merge · P push`: the same it test
  
  This does not: the drawn bar is `u undo the merge · P push · a amend · space show · …` at 120 **and** at 200. 08's `JustMerged` list has `a amend` third, and the row-bar fitter keeps `space show the 2 commits inside` at its short rung. So the box's exact words are not what draws. I did not change 08's list or the fitter (out of scope). Question for the conductor.
- [x] Fast-forward fixture (05s's): `m y` → no `JustMerged`, no `▸` — `finish::landing::a_fast_forward_lands_with_no_just_merged`.
- [x] `u` after landing: HEAD back to the pre-merge commit — `finish::landing::y_lands_on_the_merge_and_u_takes_it_back`.
- [x] Captures (colour unverified): `docs/shots/ease-pass/merge-review-120.txt`, `merge-review-90.txt` (`finish::landing::the_review_reads_the_mock_lines_and_stages_nothing`), `merge-landed-120.txt` (`finish::landing::y_lands_on_the_merge_and_u_takes_it_back`; the merge's id zeroed). m6 recaptures: `fix-y-opens-the-review-187x50`, `fix-finish-from-the-resolver-120x40`, plus `fix-finish-summary-{120x40,160x48,200x50}`, `fix-finish-summary-expanded-120x40`, `fix-summary-preview-{187x50,80x24}`, `fix-c-in-pane-finishes-187x50`, `fix-finish-shell-merge-120x40`. All under 32 KB.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~70k.

## NOTES (builder, 2026-10-18)
- **Scout used:** `.scratch/ease-pass/scouts/12-finish-review-and-look.md`. **Compacted: YES, twice.** Nothing is left over; the 3 open boxes need a decision, not more build.
- **keys.rs:** `update/finish/keys.rs` 390 → 215 lines. Every key goes by keymap id; the text-field half is gone (`subject_key`, `body_key`, `control_key`, `edit_outside`, `paste`, `close_keeping_draft`). No split needed.
- **Keymap rows deleted: none** (decision 5 named the exception). The only fields-only `Finish` row is `finish.edit_outside` (`⌃E`), and it stays: the rewrite message field acts on it (`rewrite_editor/keys/field.rs:45`) and its `$EDITOR` hint names it (`hints/lists.rs` `FIELD_EDITOR`). In the review it does nothing, listed in `sweep_swallowed.rs` and `palette_live/dead.rs`. Same for `⇥`/`⇧⇥`.
- **§2.6b edit text for the conductor** (`docs/design/02-keymap.md:431-460`): for a merge the table above the "Decided" paragraph no longer holds. The review's message is git's, read-only. `y` commits it as `git commit --no-edit` would. There is no subject/body focus: `Tab`/`Shift+Tab`, `Ctrl+E`, `Ctrl+Enter` and subject-`Enter` do nothing. `Esc` keeps no draft. `Enter`/`Space` still show a clashed file's diff inline until 12b's look; on the combined row they do nothing. Title in the frame: `All N clashes solved. Nothing committed yet.`, no `Preview: finish merging …` border title. No other operation opens the summary any more (decision 8).
- **Retiring:** `HintContext::Finish { .. }` and `finish_summary::SummaryFocus` remain only as the retiring form; nothing picks them. The sweeps map `Finish` to the review's state (`sweep_screens.rs:185`, cursor on the clashed row) and `classified_form` to `None` (`dead_keys.rs:283`).
- **11's files, minimal edits:**
  - `update/tests/sweep_screens.rs:185`: `Finish | Review` arm, last.
  - `update/tests/dead_keys.rs`: `:30` bar-hint match; `:283` `Finish` joined the existing `None` arm; `:328` `Review` joined the `Some` arm. These two were merged into existing arms, not appended, because clippy `match_same_arms` failed on separate arms.
  - `update/resolver/tests/review_at_y.rs`: `FinishFocus` out, `incoming: None`, the `⌃Z` test lost its focus loop (renamed `ctrl_z_in_the_review_undoes_the_last_pick`), `u_in_the_subject_is_typed_text` deleted (no subject).
  - `update/resolver/tests/finish_here.rs`: draft asserts out; status words.
- **Fixed on the way (cause):**
  - `update/commit_editor/done_row.rs` `bar_row` now mirrors `view/rows/context.rs`'s `landed_bar` → `just_merged_bar` order. Without it, `view/frame.rs:282`'s debug check fired after every merge landing (done line `InBar` vs `AboveBar`).
  - The review's bar hints got one-word short rungs. Without them the bar drew nothing at 47 cells (`the_floor_fits_at_the_narrowest_bar`).
- **Time until `Known` on the fixture:** 58 ms from the landing to the `JustMerged` bar (`finish::landing::y_lands_on_the_merge_and_u_takes_it_back`, printed, debug build).
- **Fast-forward status words** (observed, `a_fast_forward_lands_with_no_just_merged`): `Done { what: "merge sara/fix-toc into wallet-release" }`, drawn `✓ merge sara/fix-toc into wallet-release`. Today's words, lower-case, no full stop, unlike the review's `✓ Merged … .`. Not changed (scope: "lands as today").
- **m5 walk `summary-open-git-combined`:** it expanded README's diff from the combined row. Per decision 10 that does nothing until 12b, so the journey now goes `j j` (not counted) then `space` (counted, 1, unchanged) and checks nothing opens. 12b must restore the "README's change shown" frame. The m6 contact sheet's caption for `m6-16-summary-expanded` (`docs/shots/m6/m6-contact-sheet-captions.tsv:98`) still describes the old summary: stale, not edited.
- **Other words changed:** `merged feature into main` → `Merged feature into main.` in the m5 walk, `shell_begun`, `finish_here` and the tours. The tours wait for `Nothing committed yet.` instead of the old preview title.
- **Splits, no row raised:**
  - `hints/context.rs` 489 → 406 (`ConfirmYes` → `context/confirm_yes.rs`)
  - `hints/tests.rs` 3173 → 3114 (`blame_place` → `tests/blame_place.rs`)
  - `graph_screen/tests.rs` 2859 → 2753 (header tests → `tests/header_narrowing.rs`; `finish_slot` helper)
  - `update/finish/tests.rs` 797 → <400 (row removed; `tests/answers.rs`)
  - `hints/filters.rs` `live_bar` extracted (clippy line limit)
- **Broken vs unverified:** none known broken. **Unverified:** colour (rule 5) and the look of every capture (rule 6, the user's eye). The fast-forward words were observed, not specified.
- **Touches:**
  - callers of the removed fields: grep `finish_draft|FinishFocus` → none; `editor_returned` only in the commit editor, its own
  - `HintContext` exhaustive matches: compile
  - keymap sweeps and help dead rows: tables appended
  - `no_literal_keys` allow rows: 3 new, 2 stale removed
  - captures: listed above
  - MAP.md: graft-ui `finish_summary`, `ReviewBar`; graft `jobs::finish`, `update::finish`, `view::finish`
  - undo after a merge: it test
  - 08's `landed_bar`: its tests green, unedited
