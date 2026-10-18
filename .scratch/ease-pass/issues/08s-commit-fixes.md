# 08s — commit screen fixes (review ease-pass-08: Majors 1–7, minors 1–8, 11, 12 in part; Q1, Q2; absorbs 08r)

Status: partial — Part A built and committed (`ease-pass-08s: part A …`); Part B not started, handoff in NOTES. M6 open (two rows are 06b's).
Blocked by: 05c-merge-review-rest (its wave owns `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` and edits the clash-stop landing in `update::operate`; all edited here), 06r-fold-fixes (shared registries: `docs/man/mwr.1`, `TESTING.md`, `docs/design/02-keymap.md`, `over_400.txt`, re-shot captures). **Conductor:** 06s's Blocked by gains 08s (`update/state.rs`, `update/events.rs` lowered here); 06b also edits graft-ui `graph_screen/layout.rs` (M4 may too), so order them.
Build with: Opus (`builder-opus-xhigh`) — every decision is made below and pinned by a state, real-git or render test; no cross-crate signature for another ticket; no correctness that hides from the tests. Not Fable.
Needs: from 08 (built): `Draft`, `LastCommit`, `include_of`, `landed_bar`, `CommitEditor::from_draft`, `commit_landed`, `CommitScreenView`, `CommitFileRow`, `Summary`, `CommitHeader`, `push_words`, `whole_parts`. From 04/04r (graft-ops): `Include`, `FileChoice`, `Part` (`Part::StagedPart`), `first_choice`, `looks_like_secrets`, `commit_chosen`. graft-git `DiffSpec`. graft-ui `RowBar::{JustCommitted, JustMerged}`, `fit_status_line`.
Review: `docs/reviews/ease-pass-08.md`. Parent: `08-commit-screen.md`. Absorbs: `08r-tick-first.md` (marked absorbed).

## Read first
- `docs/reviews/ease-pass-08.md` (all).
- `.scratch/ease-pass/issues/08-commit-screen.md`: Scope C7, C9–C15, Q8, Amend, Signatures, NOTES (deviations, Touches).
- Mock `prototypes/ease-mocks/mergewright-commit-screen.html`: `:83` header, `:92` rule 1, `:116` masked secrets diff, `:146` file click, `:162` bars, `:178-179` landing (done line **and** bar).
- `docs/design-notes/ease-pass.md` :108-118 (C1–C15), :135-136 (amend), :279 (Q9 a), :281 (Q11 a: today's top-line frame).
- `docs/design/02-keymap.md` §2.10 (:521-541).
- `crates/graft/MAP.md` `update::commit_editor`, `update::operate`, `update::undo`, `update::mouse`, `update::geometry`, `view::commit_editor`. `crates/graft-ui/MAP.md` `## commit_editor` (:227-245), `graph_screen::hints` (:199-224). `crates/graft-git/MAP.md` `DiffSpec`.
- Scouts `.scratch/ease-pass/scouts/08-commit-screen.md`, `08b-*`. No new scout: the review names every line; Step 1 replaces one.

## Step 1 (before code; answers in NOTES, each with its file:line)
1. `commit_chosen` (04r) with a `Part::StagedPart` choice commits the **index's** blob for that path, not the working file. If it commits the working file: **stop, `blocked: StagedPart commits the working tree`** (M3's premise; the fix would be in graft-ops).
2. The commit screen stays open (`state.commit_editor` is `Some`) from its own launch until its write lands or fails. If it closes at launch: stop and report (M1's slot model assumes it stays).
3. The one renderer every `Status::Done` goes through; where the graph's last row is shared by the status line and the bar (graft `view/frame/bars.rs` or graft-ui `graph_screen/layout.rs`).
4. Mouse dispatch order in `update/mouse.rs:21-99`: where an open palette over the commit screen takes the mouse, and where the graph arms start.
5. Where the commit screen's diff lines for the cursor file become `SanitizedText` (`view/commit_editor.rs`); the mask goes in before that point.

## Decided here
**The promise:** what the screen says goes in is exactly what goes in; typed words are never dropped.

- **M1 — two draft slots, taken on open, given back on close.** `AppState.commit_draft: Option<Draft>` retyped in place to `drafts: Drafts` (two boxed slots: `AppState` shrinks, review m9's headroom grows).
  - `c` takes `drafts.commit` into the screen; `a` takes `drafts.amend`. Options reset to off on every open (08, unchanged).
  - `esc`/`close()` writes **only its own way-in's slot**: `c` → `drafts.commit` = the words, `None` if empty. `a` → `drafts.amend` = the words **only if they differ from what `a` prefilled**, else `None` (nothing typed).
  - An amend draft typed for an older HEAD (`Draft.amending != HEAD`): `a` still shows those words; the header names the commit it now amends (`amending <sha7> <subject>`); the screen's status row reads `your words from amending <old sha7>`. Never dropped, never silent.
  - **The screen's own write lands** (Step 1.2: the screen is open and launched it, op `Commit`/`Amend`): the screen closes, nothing written back (the words are in the commit). **Its write fails:** unchanged (screen stays, words kept). **Any other write lands:** neither slot changes and an open screen stays open with its words (`landed()` keyed on the launched op; today's `committed()` call at `operate/outcome.rs:137` runs only for the screen's own).
  - **Q9 `u` (unchanged in meaning):** `undo_landed` takes `last_commit` and gives its words to `drafts.commit` **only when a ref moved off the commit** (m11: `refs_before[r] == Some(c) && refs_after[r] != Some(c)`). If `drafts.commit` already holds words (possible after an `a` screen with amend turned off committed), they stay first; the returned subject and body follow in the body after a blank line. Both show on the next `c`.
  - **m1 redo:** a redo re-creating that commit clears `drafts.commit` only if it equals the commit's message exactly. **m2 incomplete undo:** runs the same `undo_landed` as a complete one.
- **M2 — the rows are the commit.** `include_of` keys on how the screen was opened, not on the amend flag: `Include::Staged` **only** for a screen opened by `a`, still amending, rows untouched (then the rows going in are the index). Every other case → `Include::Chosen` from the rows going in. So `c` + `⌃P` amend amends **with the rows shown**; `a` + amend turned off makes a new commit of the rows shown.
  - Amend turned on from a `c` screen: header `amending <sha7> <subject>` from what the state already holds for HEAD (oid from `HeadFacts`; subject from HEAD's loaded row, else `amending <sha7>`); no new read. Summary ends `into the last commit.` Amend turned off on an `a` screen: header `committing to <branch>`, summary `as a new commit on <branch>.` The message box is the message in both (WYSIWYG).
  - With amend on from `c` and nothing going in: `put a file in first`, as a commit (a message-only amend is `a`'s).
- **M3 — show the part that goes in.** Each row's counts and diff follow its `FileChoice`'s `Part`: `Part::StagedPart` → index vs HEAD (the diff's staged section; the counts read the matching `DiffSpec`); any other `Part` → HEAD vs working tree, as today. Same rule for a row staying out (it shows what `space` would put in). The summary sums these.
  - A `StagedPart` row draws a dim ` part` after its counts; its diff starts `only the part you staged goes in; the rest stays uncommitted.` (the `! left out:` line's place and style). Different things look different (field rule).
- **Q1 — a secrets file's values are hidden (build to the mock, `:116`).** For a row flagged secret (`CommitFileRow.secret`, from `looks_like_secrets`), every added, removed and context line of its diff on the commit screen draws its value as `••••••••` (eight, whatever the real length): the text after the line's first `=` or `:` is replaced, the key kept (`PAYMENT_KEY=••••••••`); a line with neither is `••••••••` whole. Hunk headers and the `! left out:` line stay as they are. The flag goes with the file: in or out, masked. Counts unchanged (they count lines, not values).
  - Masked in graft before the text becomes `SanitizedText` (Step 1.5): the real value never reaches graft-ui, a view, a capture or a log.
  - Out: the changes pane and detail diff elsewhere in the app (not the commit screen).
- **m3 + m10 (cancel half):** while the screen is open, every working-tree re-read re-asks the counts and the cursor file's diff, cancelling the counts read still running. The commit takes the files as they are; the screen shows them as they are.
- **M4 + 08r — the landing is the mock's two lines.**
  - Every `Status::Done` renders `✓ <what>  <keys>` (tick first, two spaces, no `·`; mock :178), through the one renderer. Cut to fit as today.
  - While a `Done` status stands **and** the cursor row's bar is a landing bar (`RowBar::JustCommitted`; `JustMerged` gets the same if any app code picks it — say which in NOTES), the done line takes its own row directly above the bar; the graph gives up that row. Any other status: in the bar's row, as today. A key that clears the status gives the row back.
  - Result at 120: second-last row `✓ Committed 3 files to main.  u undo · P push`; last row `u undo the commit, your edits come back · P push · c commit again`.
- **M5 + Q2 — the commit screen owns the mouse.** One arm, after any overlay drawn over the screen (palette), before every graph arm: left click on a file row → focus Files, cursor to it, its diff read (what `j`/`k` do); on the subject → focus Subject; on the body → focus Body (caret unmoved). Every other event while the screen is up — heading rows, summary, header, bar, right click, drag, wheel — is swallowed: no graph cursor move, no menu, no drag. One layout fn in graft-ui both draws and hit-tests, so they cannot drift.
- **M6 — split, don't raise** (user, 2026-10-18). `update/commit_editor.rs` → `commit_editor/options.rs` (`CommitOption` + impl, :46-97), `commit_editor/open.rs` (`open`, `show`, `head_message_arrived`, :330-461); root under 400, its `over_400.txt` row removed. The other rows 08 raised go back to their pre-08 value or lower by moving 08's additions (or a cohesive neighbour) into new modules: `update/operate/outcome.rs` ≤415 (m7's `after_landing.rs` does it), `update/events.rs` ≤598, `update/events/command.rs` ≤484, `view/frame/bars.rs` ≤413, `view/frame/boxes.rs` ≤460, graft-ui `graph_screen/layout.rs` ≤650, `graph_screen/mod.rs` ≤573. `update/state.rs` and `view/rows.rs` are 06s's/06t's: this ticket adds no net line to either. No row raised or added; new code (M4, M5, Q1) goes in new modules.
- **M7:** `head_message_arrived` keeps guard-and-dispatch; `amend_screen(state, head, message, reword_line) -> CommitEditor` and `resume_amend_draft(state, &mut editor, head)` in `open.rs`. Nesting ≤3.
- **m4 narrow bar:** with the diff hidden (< `DIFF_COLUMNS`) the subject bar omits `↑ ↓ see a file's changes`. Short of room, parts drop by rank, lowest first: `↑ ↓` < `tab more detail` < `esc cancel, your message is kept` < `⏎ …`. Never cut mid-word.
- **m7:** one `after_landing(state, launched)` runs `landed` + `amend_into::landed` + `recover::tag_rows` + `commit_landed` for all three landing paths (`outcome.rs:33-42, :251-277, :297-315`); the incomplete path drops the commit's done words explicitly, with a comment why.
- **m12 (header half):** the screen's header starts with the repository words the graph's top line starts with (same fn, same cut; Q11 a: today's frame, not the mock's `mwr`), then `committing to <branch>`, then the push state. The path cuts first when narrow.
- **m5, m6, m8:** caption, asserts, docs: Acceptance.

## Scope
In — graft app: `update/commit_editor.rs` and `commit_editor/{keys,files,landing}.rs`, new `commit_editor/{options,open,mouse}.rs`; tests `commit_editor/screen_tests.rs` (or a sibling `draft_tests.rs` if it would cross 400); `update/state.rs` (one field retyped, in place); `update/operate/outcome.rs` + new `operate/after_landing.rs`; `update/undo/answers.rs` (:171-172, :212-217); `update/mouse.rs` (one arm); `jobs/kinds.rs` (`read_commit_files` by `Part`; per-path counts); `view/commit_editor.rs` + new `view/commit_editor/mask.rs` (Q1); the graph bottom-row site (Step 1.3) and whatever moves out of the seven raised files; `tests/it/commit_screen.rs`, `tests/it/commit_editor_writes.rs` (m8 helper), `tests/it/over_400.txt` (rows lowered/removed only). graft-ui: `commit_editor` (row ` part`, diff top line, header repo words, bar rank, layout + hit-test), the `Done` renderer, `graph_screen::hints` (`status_row`, `RowBar::is_landing`). Docs: `TESTING.md` :1419-1421, :1473-1484, :1537; `docs/design/02-keymap.md` :527 (text below); `docs/shots/ease-pass/commit-files-focus-120.txt:4` caption; MAP.md rows for every new item; captures.
Out: graft-ops (Step 1.1 stops instead); hint list words and keymap rows; `view/rows.rs`, `update/state.rs` splits (06s/06t); masking outside the commit screen; review m9 (maker closures), m10's per-file failure half, wheel scrolling on the screen: KNOWN_GAPS rows (conductor).

**02-keymap.md :527 (verbatim):** `| \`Tab\` | Move subject → body → files → subject (\`Shift+Tab\` back) |`

## Signatures
graft app, `update::commit_editor`:
```rust
/// How the screen was opened. A toggle from `⌃P` never changes it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WayIn { Commit, Amend }

/// Words a closed commit screen keeps this run, one slot per way in (`AppState.drafts`, replaces `commit_draft`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drafts { pub commit: Option<Box<Draft>>, pub amend: Option<Box<Draft>> }

impl CommitEditor { pub fn way_in(&self) -> WayIn; }

/// `Staged` only for `WayIn::Amend`, `amend` on, `touched` false. Otherwise `Chosen` from the rows going in.
pub fn include_of(choices: &[FileChoice], way_in: WayIn, amend: bool, touched: bool) -> Include;

/// The undone commit's words into the `c` slot; words already there stay first.
pub fn give_back(drafts: &mut Drafts, words: Draft);

/// The section a row's counts and diff read: what goes in. `StagedPart` → the staged section.
/// (`DiffSection` = the type `DiffKey.section` holds; use its real name.)
pub fn shown_section(part: &Part) -> Option<DiffSection>;

/// Every mouse event while the screen is up; none reaches the graph. Return type as `update/mouse.rs`'s arms.
pub(crate) fn mouse(state: &mut AppState, event: &MouseEvent) -> Vec<Command>;
```
graft app, `view::commit_editor::mask`:
```rust
/// The mask a secrets file's value is drawn as.
pub(crate) const MASK: &str = "••••••••";
/// One diff line of a secrets-looking file: the key kept, the value after the first `=` or `:` → `MASK`;
/// no separator → `MASK`. Runs before sanitising.
pub(crate) fn mask_secret_line(line: &str) -> String;
```
graft app, `update::operate`:
```rust
/// The one post-landing hook list. `Some` = the commit's done status, for the caller to show or drop.
pub(crate) fn after_landing(state: &mut AppState, launched: &Operation) -> Option<Status>;
```
graft-ui `commit_editor`:
```rust
pub struct CommitLayout { pub header: Rect, pub subject: Rect, pub body: Rect, pub summary: Rect,
    pub files: Rect, pub diff: Option<Rect>, pub bar: Rect }
/// `CommitScreen` draws from it; the app hit-tests with it.
pub fn commit_layout(area: Rect) -> CommitLayout;
/// The file under `row`; heading and `nothing` rows → `None`. Honours the list's scroll.
pub fn file_at(view: &CommitScreenView, files: Rect, row: u16) -> Option<usize>;
// CommitFileRow gains `pub part_only: bool`; CommitHeader gains `pub repo: SanitizedText`.
```
graft-ui `graph_screen::hints`:
```rust
impl RowBar { /// `JustCommitted` | `JustMerged { .. }`: the done line stands above this bar.
    pub fn is_landing(&self) -> bool; }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusRow { InBar, AboveBar }
pub fn status_row(done: bool, context: &HintContext) -> StatusRow;
```

## Touches (check each, report how)
- `include_of`, `close()`, `committed()`, `landed()` callers (grep); the three landing paths; `undo_landed`, redo and `Undone::Incomplete` arms; the journal screen's undo (same answers path).
- 08's tests: `esc_keeps_subject_and_body_…`, `an_amend_draft_resumes_on_a_not_on_c`, `u_after_the_commit_gives_its_message_to_the_next_c`, `an_amend_names_its_commit_and_takes_the_index_until_space`, `the_secrets_file_stays_out_marked_and_space_puts_it_in`, graft-ui `the_secrets_files_diff_starts_with_why_it_is_left_out`, `palette::editor_tests::choosing_each_option_twice_returns_it_to_off` — pass, edited only for the `drafts` rename or the masked text (say which).
- Journeys `commit-all-subject` 3, `commit-with-body` 6, `commit-leave-first` 6: unchanged counts.
- Q1: every capture and test fixture holding a secrets file's value (`git grep -n 'TOKEN=\|PAYMENT_KEY=' docs/shots crates/*/tests`): re-shot masked; no real value left (grep in NOTES).
- Every `Status::Done` producer and every capture or pty waiter holding a done line (`grep -rn ' ✓ ·\|✓ ·' docs/shots crates/*/tests`): re-shot, each listed in NOTES as tick-place-only; any other diff → stop and report (08r's D3 rule). Man page / capabilities if they quote a done line.
- Graph bottom rows at 80×24 and 120×40 after a commit; detail pane open and closed.
- Mouse: palette over the screen still takes clicks; the context menu and badge drag never open under the screen.
- Finish summary (`view/finish.rs`) shares nothing changed here: check by its tests.
- `over_400.txt` (`git diff`: rows lowered or removed only); clippy `large_stack_arrays` (AppState size before/after in NOTES); MAP coverage; `no_literal_keys` (new words through `key_verb`).
- 06s/06t line refs into `state.rs`: NOTES gives the new home of `commit_draft`.

## Acceptance
Each tick names its test inline (names are proposals; a rename is said in NOTES).
- [x] **M1 (a):** `c`, `half`, `esc`; `a`, answer, `esc`; `c` → subject `half`. — unit `draft_tests::a_peek_at_the_last_commit_keeps_the_c_words` — **ticked:** `update::commit_editor::draft_tests::a_peek_at_the_last_commit_keeps_the_c_words` passes
- [x] **M1 (a) reverse:** `a`, edit to `better`, `esc`; `c`, `esc`; `a` → `better`. `a`, `esc` unchanged → `drafts.amend` `None`. — unit `…::an_empty_c_screen_keeps_the_amend_words`, `…::an_unchanged_amend_keeps_no_words` — **ticked:** `draft_tests::an_empty_c_screen_keeps_the_amend_words`, `draft_tests::an_unchanged_amend_keeps_no_words` pass
- [x] **M1 older HEAD:** amend words kept, a commit lands, `a` → those words, header `amending <new sha7>`, status row `your words from amending <old sha7>`. — unit `…::amend_words_for_an_older_head_come_back_on_a_and_say_so` — **ticked:** `draft_tests::amend_words_for_an_older_head_come_back_on_a_and_say_so` passes (header `0808080 next`, status `your words from amending 0707070`)
- [x] **M1 (b):** a kept `c` draft; a stage write lands through `update` → `drafts` equal. Screen open, a fetch lands → screen open, words equal. Screen's own commit lands → screen closed, `drafts.commit` `None`. — unit `…::another_write_landing_keeps_the_words`, `…::another_write_landing_keeps_the_open_screen`, `…::the_screens_own_commit_closes_it_and_keeps_nothing` — **ticked:** `draft_tests::another_write_landing_keeps_the_words`, `draft_tests::another_write_landing_keeps_the_open_screen`, `draft_tests::the_screens_own_commit_closes_it_and_keeps_nothing` pass
- [x] **Q9 + give_back:** `u` after a commit with `drafts.commit` holding `half` → next `c`: subject `half`, body holds the commit's subject and body after a blank line. — unit `…::undo_gives_back_without_replacing_typed_words`; it `commit_screen::u_after_the_commit_gives_its_message_to_the_next_c` passes — **ticked:** `landing_tests::undo_gives_back_without_replacing_typed_words` and it `commit_screen::u_after_the_commit_gives_its_message_to_the_next_c` pass
- [x] **m1:** `u`, `U` → `drafts.commit` `None`; `u`, edit the draft, `U` → kept. — unit `…::redo_takes_back_only_the_unchanged_returned_words` — **ticked:** `landing_tests::redo_takes_back_only_the_unchanged_returned_words` passes
- [x] **m2 + m11:** incomplete undo that moved the ref off → words given back, `last_commit` `None`; a ref equal before and after → `last_commit` kept, nothing given. — unit `…::an_incomplete_undo_gives_the_words_back`, `landing::tests::undo_landed_needs_the_ref_moved_off_the_commit` — **ticked:** `landing_tests::an_incomplete_undo_gives_the_words_back`, `landing::tests::undo_landed_needs_the_ref_moved_off_the_commit` pass
- [x] **M2:** real git, `c`, type `fix toc`, `⌃P` amend, `⏎` → HEAD's parent unchanged, HEAD's message `fix toc`, its tree holds the 3 shown files, `.env` out; header read before `⏎` is `amending <sha7> <subject>`, summary ends `into the last commit.` `a` + amend off + `⏎` → a new commit of the rows shown. — it `commit_screen::amend_from_ctrl_p_amends_with_the_rows_shown`, `…::amend_turned_off_commits_the_rows_shown`; unit `include_of_table` (every `WayIn` × `amend` × `touched`) — **ticked:** it `commit_screen::rows::amend_from_ctrl_p_amends_with_the_rows_shown`, `commit_screen::rows::amend_turned_off_commits_the_rows_shown`, unit `commit_editor::files::tests::include_of_table` (8 literal rows) pass
- [x] **M3:** real git, `src/a.rs` with one staged and one unstaged added line → row `+1` and ` part`, summary `+1 −0`, diff holds the staged line only and starts with the `only the part you staged…` line; `⏎` → HEAD's diff for `src/a.rs` is that line; the unstaged line still in the file. — it `commit_screen::a_partly_staged_file_shows_and_commits_only_its_staged_part`; unit `…::a_staged_part_reads_the_staged_section`; graft-ui `a_staged_part_row_says_part` — **ticked:** it `commit_screen::rows::a_partly_staged_file_shows_and_commits_only_its_staged_part`, unit `screen_tests::a_staged_part_reads_the_staged_section`, graft-ui `commit_editor::part_tests::a_staged_part_row_says_part` (note level with the first heading: the diff's top line) pass
- [x] **Q1 lines:** `mask_secret_line`: `PAYMENT_KEY=abc123` → `PAYMENT_KEY=••••••••`; `token: abc` → `token:••••••••` (the key and separator kept, the rest masked); `a=b=c` → `a=••••••••`; `abc123` → `••••••••`; `` → `••••••••`; a 200-char value → 8 bullets. — unit `view::commit_editor::mask::tests::a_secret_line_keeps_its_key_and_hides_its_value` — **ticked (08t):** `view::commit_editor::mask::tests::a_secret_line_keeps_its_key_and_hides_its_value` passes (all six rows; seen red first, an assertion failing at `mask.rs:31` against a stub)
- [x] **Q1 screen:** real git, `.env` = `TOKEN=s3cr3t-value-q1`; `c`, cursor on `.env` → its diff line reads `+ TOKEN=••••••••`; `space` puts it in → still masked; `CommitScreenView` (every `SanitizedText` in it, by a `Debug` dump) holds no `s3cr3t`; a non-secrets file's diff unmasked. — it `commit_screen::a_secrets_files_values_never_reach_the_screen`; unit `…::a_secrets_row_is_masked_in_and_out`, `…::an_ordinary_file_is_not_masked` — **ticked (08t):** it `commit_screen::a_secrets_files_values_never_reach_the_screen` (out, then in after `⇥ ⇥ space`: frame has `+ TOKEN=••••••••`, frame and `Debug` of `commit_screen_view` lack the value; README.md's `+ more` unmasked), unit `update::commit_editor::mask_tests::a_secrets_row_is_masked_in_and_out`, `…::mask_tests::an_ordinary_file_is_not_masked` pass; each seen red first
- [x] **Q1 captures:** `commit-secrets-120.txt` re-shot shows `TOKEN=••••••••`; `git grep -n 's3cr3t' docs/shots` and the old raw value `git grep -n 'TOKEN=x' docs/shots` both empty. — `commit_screen::shots::capture_the_commit_screen` asserts the written file lacks the fixture value; grep in NOTES — **ticked (08t):** `commit-secrets-120.txt` re-shot, its diff column `+ TOKEN=••••••••`; `commit_screen::shots::capture_the_commit_screen` asserts no frame row holds `SECRET_VALUE`; `grep -rn 's3cr3t' docs/shots` → 0, `grep -rn 'TOKEN=x' docs/shots` → 0 (08t NOTES)
- [x] **m3 + m10:** a re-read with the same set of files → a new counts read asked, the old one cancelled, the cursor diff re-asked. — unit `…::a_reread_while_open_reads_counts_and_diff_again` — **ticked:** `screen_tests::a_reread_while_open_reads_counts_and_diff_again` passes
- [x] **08r:** a commit's done line renders `✓ Committed 3 files to main.  u undo · P push`; every `Done` producer renders tick first through one renderer. — graft-ui `the_done_line_leads_with_its_tick`; one table test over the producers or the grep proving one renderer (NOTES) — **ticked (08t):** `view::status::tests::the_done_line_leads_with_its_tick` (the renderer is graft's `view/status.rs done_line`, not graft-ui: name corrected) and `…::done_line_places_also_before_keys` pass, seen red first; one renderer by grep: the only non-test `✓` beside a `Status::Done` is `view/status.rs:65` (08t NOTES)
- [ ] **M4:** (Part B — not built) after a commit at 120×40: second-last row the done line, last row `u undo the commit, your edits come back · P push · c commit again`; a key that clears the status → bar alone, graph one row taller; a `Done` off a landing bar → in the bar's row. — graft-ui `status_row_is_above_only_for_a_landing_bar`; graft `view::frame::tests::a_commit_landing_draws_the_done_line_above_its_bar`
- [ ] **M5 + Q2:** (Part B — not built) click a file row → Files focus, cursor there, its diff asked; click a heading → nothing; click subject/body → focus there; left click on a graph row's place, right click, drag, wheel → graph cursor, menu and drag state unchanged. Palette open → it takes the click. — unit `…::mouse::a_click_on_a_file_row_picks_it`, `…::mouse::clicks_in_the_message_move_focus`, `…::mouse::nothing_reaches_the_graph_under_the_screen`, `…::mouse::the_palette_over_the_screen_takes_the_click`; graft-ui `file_at_skips_headings_and_follows_the_scroll`
- [ ] **M6:** (open: 6 of the 8 rows done, see NOTES; `graph_screen/layout.rs` 653 and `mod.rs` 578 are held by 06b, not lowered) `commit_editor.rs` < 400, row gone; the seven rows at or under their pre-08 values; no row raised or added. — it `file_size` ratchet; `wc -l` + `git diff over_400.txt` in NOTES
- [x] **M7:** `head_message_arrived` guard-and-dispatch, nesting ≤3. — by reading, line count in NOTES; clippy `cognitive_complexity` clean — **ticked:** `open.rs:76-113`, 38 lines, guards at depth 1, nothing deeper than the `match` arm (depth 2); clippy `-D warnings` with `cognitive_complexity = "warn"` (root `Cargo.toml:59`) clean
- [ ] **m4:** (Part B — not built) at 90 columns the subject bar has no `↑ ↓` and keeps `esc cancel, your message is kept`; at a width fitting two parts: `⏎` and `esc`. — graft-ui `the_narrow_bar_drops_by_rank`
- [x] **m6:** `undoing_something_else_leaves_the_draft_alone` seeds `Some`; `another_write_landing_clears_the_last_commit` lands a stage `Operated` through `update`. — those two tests (diff in NOTES) — **ticked:** `landing_tests::undoing_something_else_leaves_the_draft_alone`, `landing_tests::another_write_landing_clears_the_last_commit` pass
- [x] **m7:** a clean and an incomplete commit landing run the same hooks; `recover::tag_rows` called only in `after_landing`. — unit `…::every_landing_path_runs_the_same_hooks`; grep count in NOTES — **ticked:** `draft_tests::every_landing_path_runs_the_same_hooks` passes (Done, Incomplete, StagingNotKept)
- [x] **m8:** `TESTING.md` :1419-1421, :1473-1484, :1537 describe `⏎` from the subject and today's refusal words (as the walk's tests assert; name them); 02-keymap :527 as above; `press_the_button` renamed for what it does; if `keymap/table/commit_editor.rs`'s `Tab` description names options, reworded and the man page regenerated (diff in NOTES). — `git grep -n '\[ Commit \]' TESTING.md docs/design` empty — **ticked (08t):** `grep -rn '\[ Commit \]' TESTING.md docs/design` → empty; TESTING.md items name `m3_walk::every_scenario::{commit,commit_refused_nothing_staged,amend}`; 02-keymap :527 verbatim (cycle checked against `update::commit_editor::tests::tab_walks_subject_body_files_and_back`); `press_the_button` → `tab_to_the_subject_and_enter` (also in `m3_walk.rs`, `m4_walk/run.rs`); `Tab` reads `Next field` — no reword, man page unchanged
- [ ] **m12:** (Part B — not built) header at 120 reads `<repo words>  committing to main  1 commit to push`-shaped (the repo words as the graph's top line); at 80 the path is cut first. — graft-ui `the_header_names_the_repository_first`
- [x] **m5:** `commit-files-focus-120.txt` caption reads `⏎ commit`. — grep in NOTES — **ticked (08t):** caption reads `⏎ commit  esc back to the message`, re-shot by `commit_screen::shots::capture_the_commit_screen` (grep in 08t NOTES)
- [ ] Captures (look unjudged, rule 6; colour unverified, rule 5), each with its "what right looks like" line: re-shot `commit-landed-120.txt` (two rows as M4), `commit-open-120.txt` (repo words), `commit-90.txt` (narrow bar), `commit-secrets-120.txt` (masked); new `docs/shots/ease-pass/commit-partial-120.txt` (M3), `commit-amend-from-c-120.txt` (M2 header and summary); every done-line capture re-shot (list). — `commit_screen::shots::capture_the_commit_screen` and each writer named in NOTES — **open (08u):** 08t did `commit-secrets-120.txt` (masked) and every done-line re-shot (list in 08t NOTES; tick place only). `commit-landed-120.txt`'s two rows, `commit-open`, `commit-90`, `commit-partial`, `commit-amend-from-c` are 08u's.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~100k: over one context. **Split before dispatch:** A = M1, M2, M3, M6, M7, m1, m2, m3, m6, m7, m10, m11 (words and what goes in); B = Q1, M4 + 08r, M5 + Q2, m4, m5, m8, m12 (what is drawn, and the mouse). B after A (both touch `view/commit_editor.rs` and graft-ui `commit_editor`). This file's Decided, Signatures and Touches hold for both.

## For KNOWN_GAPS (conductor; ready rows in the planner's scratch file)
m9 headroom, m10 per-file failure, m12 unverified items, secrets shown outside the commit screen, wheel on the screen.

## NOTES (builder, part A — 2026-10-18)

**Compacted twice** mid-ticket. Part A is built and committed. Part B is not started; the handoff is below. **No scout:** the ticket says "No new scout: the review names every line", and Step 1 stands in for one.

### Step 1 answers (line numbers at base `b7168e5e`)
1. **`StagedPart` commits the index's blob.** `crates/graft-ops/src/operation/commit.rs:62-81, :91-95, :114-116`; test `commit_chosen.rs:145`. Not blocked.
2. **The screen stays open while its write runs.** With `state.operating` set, `update/commit_editor/keys.rs:23-31` takes every key (`Esc` stops the write); `paste` at :191 does the same. It closes only in `close()`, in `committed()`, and at the reword take. Not blocked.
3. **Done renderer and last row.**
   - One renderer: `view/status.rs:62 done_line`. Two draw sites: `view/frame/bars.rs:314` (now `view/frame/notices.rs:49`) and `view/stash.rs:101`.
   - The graph's last-row sharing (M4) is not traced. That is Part B.
4. **Mouse dispatch order** (`update/mouse.rs`):
   1. palette at :28
   2. the modal chain at :42-47 (`finish_mouse`, then `rewrite_mouse`)
   3. place at :52
   4. resolver at :58
   5. picker at :65
   6. preview at :68
   7. context menu at :74
   8. badge drag at :84
   9. the graph arms at :88-99 (wheel, left, right, divider drag, hover)

   The commit screen is in none of them, so its clicks fall through to the graph (Q2). Plan: add `|| super::commit_editor::mouse(…)` to the chain at :42.
5. **The premise is wrong.** The diff text is sanitised in the job: `jobs/detail/assemble.rs:170-181 line_text`. The commit screen receives `(view, raw FileDiff)` at `update/answer.rs:87-92`, already sanitised. Q1's mask therefore goes in at arrival, in `update/commit_editor/files.rs diff_arrived`, when the file is a secrets choice. `AppState` never holds the raw value past that point.

### Built (part A)
- **M1 — two draft slots.**
  - `state.commit_draft` becomes `state.drafts: Drafts { commit, amend, given_back }`, each `Option<Box<…>>`. **For 06s/06t:** this is the new home of `commit_draft`; the field stays in place at `state.rs:255-259`, net 0 lines.
  - `CommitEditor` gains `way_in: WayIn` and `prefilled`. `close()` keeps words only when `words_typed()`.
- **M2.** `include_of(choices, way_in, amend, touched)` returns `Staged` only for `Amend && amend && !touched`. `message_only()` drives both the refusal and the summary.
- **M3.**
  - `shown_section(&Part)` picks what counts and diffs read: `ReadCommitFiles(id, staged_parts)` counts those paths from the index (`jobs/kinds.rs`), and the diff asks for `section: Some(Staged)`.
  - The row's ` part` shows only when the file also has an unstaged rest, because `first_choice` marks every staged file `StagedPart` (deviation).
  - The note is the diff pane's top line.
- **M7.** `open.rs`: `head_message_arrived` (guard-and-dispatch, 38 lines), `amend_screen`, `resume_amend_draft`. The older-`HEAD` words note is `Status::sticky`.
- **m1, m2, m11, Q9.**
  - `give_back` keeps typed words first and puts the commit's text in the body.
  - `redo_landed` / `redo_took_back` take back only unchanged words; this needed the `Drafts.given_back` field (deviation).
  - `undo_landed` acts only when a ref moved off the commit. Otherwise it keeps `last_commit` (m11).
  - Both `Undone` arms call `commit_words` (`undo/answers.rs:155, :180, :224`).
- **m7.** `operate/outcome/after_landing.rs after_landing(state, launched, applied, lands, next_id) -> (Vec<Command>, Option<Status>)`.
  - The signature is wider than the ticket's.
  - It lives under `outcome/`, because `operate.rs` is at exactly 400.
  - Grep count: `tag_rows(` 1, `commit_landed(` 1, `amend_into::landed(` 1, all in `after_landing.rs:27-29`.
  - The incomplete and staging-not-kept paths drop the done words with `_commit_words` and a comment. The staging-not-kept path now also runs `commit_landed` (it did not before).
- **m3, m10.** `tree_read` always resets the diff and re-asks the counts and the diff. `ask_counts` cancels the old count read.
- **m6.**
  - `undoing_something_else_leaves_the_draft_alone` seeds `drafts.commit = Some("half")` and asserts the draft is unchanged. It no longer asserts `last_commit` is `None`: under m11 it is kept.
  - `another_write_landing_clears_the_last_commit` lands `Operated::Done` for a Stage through `update`.
- **M6 sizes** (`wc -l`):

  | File | Before | Now | Row in `over_400.txt` |
  |---|---|---|---|
  | `update/commit_editor.rs` | 489 | 331 | removed |
  | `commit_editor/screen_tests.rs` | 483 | 368 | removed |
  | graft-ui `commit_editor/tests.rs` | 412 | 280 | removed |
  | `operate/outcome.rs` | 430 | 388 | removed |
  | `view/frame/bars.rs` | 418 | 331 | removed |
  | `update/events.rs` | 603 | 576 | lowered |
  | `events/command.rs` | 487 | 457 | lowered |
  | `view/frame/boxes.rs` | 462 | 439 | lowered |

  - Rows were only removed or lowered; none was added.
  - New modules:
    - `events/{commit_graph_offer,take}.rs`
    - `frame/notices.rs` (`footer_notice`, `first_launch_line`)
    - `frame/commit_screen.rs`
    - `commit_editor/{drafts,open,options}.rs` and `draft_tests.rs`, `landing_tests.rs`, `files/tests.rs`, `landing/tests.rs`
    - graft-ui `commit_editor/{fixture,part_tests}.rs`
  - **Not done:** graft-ui `graph_screen/layout.rs` (653, needs ≤650) and `mod.rs` (578, needs ≤573). 06b holds both files. The M6 box stays open.
- **AppState** is 7968 bytes now, measured with a temporary `size_of` test that was then removed. Before it was about 8032: `Option<Draft>` (88) became `Drafts` (24). That figure is computed, not measured at base.

### Deviations from the ticket
- `Drafts.given_back` was added for m1.
- `after_landing`'s signature and file are as above.
- `part_only` needs an unstaged rest.
- `include_of_table` lives in `files::tests`.
- m11 keeps `last_commit` when no ref moved off the commit.
- The edit-stop `Err` path keeps its sticky, and the guard order is unchanged.

### TDD
Part A was mostly written code-first, before the second compaction. Only `a_staged_part_row_says_part` was observed red. Every other test was written alongside the code and passed when first run, so the expected values come from the ticket's literals, not from the code.

### Touches (how each was checked)
- **Callers of `include_of`, `close()`, `committed()` and `landed()`:** grep. The old `landed()` is gone; `after_landing` is its only successor.
- **The landing paths and the undo answers:** the three `Operated` paths are covered by `every_landing_path_runs_the_same_hooks`. `Product::Undone` has one arm (`update/answer.rs:208`), so the journal screen's undo takes the same path.
- **08's tests**, all passing in the full run:

  | Test | Change |
  |---|---|
  | `esc_keeps_subject_and_body_…` | moved verbatim to `draft_tests` |
  | `an_amend_draft_resumes_on_a_not_on_c` | moved, the `drafts` rename, rewritten through the `amend_open` helper (same keys and answer), and one assert added (`Status::Ready`) |
  | `u_after_the_commit_gives_its_message_to_the_next_c` | unchanged |
  | `an_amend_names_its_commit_and_takes_the_index_until_space` | unchanged |
  | `the_secrets_file_stays_out_marked_and_space_puts_it_in` | unchanged |
  | graft-ui `the_secrets_files_diff_starts_with_why_it_is_left_out` | unchanged (moved with `tests.rs`'s fixture split) |
  | `choosing_each_option_twice_returns_it_to_off` | unchanged |

  The other rename edits: `update/commit_editor/tests.rs:716, :728`, `tests/it/commit_editor_writes.rs:210`, `tests/it/resolver/editor_words.rs:241, :245`.
- **Journeys** `commit-all-subject` 3, `commit-with-body` 6, `commit-leave-first` 6: their tests (`screen_tests.rs:160-230`) are untouched by the diff and pass.
- **Finish summary:** its tests pass in the full run.
- **`over_400.txt`:** rows only removed or lowered (`git diff`). `file_size` passes.
- **MAP.md:** rows updated for graft `update::commit_editor` and `update::events`, and for graft-ui `CommitFileRow`. `map_coverage` passes.
- **`no_literal_keys`:** passes; the fixture's panic words were reworded.
- **Captures:** `docs/shots/m3-commit-editor-{160x48,80x24,rejected}.txt` were regenerated (M3). They now show ` part` on `src/auth/token.rs` and the part note as the diff's top line. **Look unjudged (rule 6); colour unverified (rule 5).**

### Gates (part A)
- fmt and clippy `-D warnings`: clean.
- nextest, workspace-wide: 6815/6817.
  - The 2 failures were mine: a MAP line over 240 characters, and the fixture's `a test backend…` panic words.
  - Both are fixed. A rerun of `map_*`, `no_literal_keys`, `file_size` and the part test passed (192/192).
- `cargo test --doc`: passes.

### Handoff: Part B (not started)
Next step, Q1 first:
- `view/commit_editor/mask.rs` gets `MASK` and `mask_secret_line`, with the unit table from the ticket.
- In `files.rs diff_arrived`, when the cursor file's choice `looks_like_secrets`, map each Added/Removed/Context line's text to `sanitize_line(&mask_secret_line(text))` and clear its `words` and `syntax`.

Then, in order:
1. **08r:** `done_line` → `✓ <what>[ · <also>]  <keys>`, then re-shoot every capture matching `grep -rn '✓ ·\| ✓ ·' docs/shots crates/*/tests`.
2. **M4:**
   - graft-ui `hints/status_row.rs`: `RowBar::is_landing`, `StatusRow`, `status_row`. `hints/context.rs` is in the ratchet at 489.
   - graft `view/frame`: the graph area loses a row while a landing bar and a Done line share it.
   - If `graph_screen/layout.rs` is truly needed, report `blocked: needs layout.rs`.
3. **M5/Q2:** a new `update/commit_editor/mouse.rs`; graft-ui `commit_layout` and `file_at`. Keep `mouse.rs` ≤400; it is at 399.
4. **m4, m5, m8, m12**, then the captures.

Gotchas:
- The worktree guard refuses complex bash. Use the Edit tool or a JSON-pairs replace script.
- The app crate's package is `mergewright`.
- `JustMerged`: whether app code ever picks it is not checked yet.

## NOTES (builder, part B1 = ticket 08t — 2026-10-18)

**Compacted once** mid-ticket (08t's session), after Q1 was committed and 08r's re-shoots were done. Nothing of 08t's scope remains. 08u's boxes (M4, M5+Q2, m4, m12, M6) are untouched. **No scout:** 08s's Step 1 answers stand in, as the conductor said.

### Test-first, what was seen red
- Q1: `a_secret_line_keeps_its_key_and_hides_its_value` (assert at `mask.rs:31`), `a_secrets_row_is_masked_in_and_out` (`mask_tests.rs:70`), `an_ordinary_file_is_not_masked`, it `a_secrets_files_values_never_reach_the_screen`: each failed before its code.
- 08r: `the_done_line_leads_with_its_tick` and `done_line_places_also_before_keys` failed before `done_line` changed. The ~40 existing tests that quote a done line then failed on the old literal and were rewritten to the tick-first one.

### Deviations
- **Hunk headings are masked too** (spec: "Hunk headers … stay as they are"). git's default funcname copies the nearest `KEY=value` line into the `@@ … @@ <heading>`. Checked with real git: `.env` = `A_KEY=first-secret`, six lines, `B=2`→`B=3` gives `@@ -5,4 +5,4 @@ A_KEY=first-secret`. The ranges stay; only the heading text goes through `mask_secret_line`. Pinned by `mask_tests::a_secrets_row_is_masked_in_and_out` (`OLD_KEY=••••••••`).
- **08r's test is in graft, not graft-ui:** the one renderer is graft's `view/status.rs done_line`.
- **Staging-not-kept** (a `Sticky`, not a `Done`): rungs 1-2 are now built from `done_line`, so they lead with the tick too. Its key keeps ` · ` because a Sticky draws `reason · keys`.
- **`done_width`** (`update/undo/consequence.rs`) counts the new shape: `2 + what + (2 + keys)`. The 80-column cut still keeps the key (`consequence::tests::the_time_machine_landing_fits_80_columns_and_keeps_its_key`).
- **m8, wider than named:**
  - The same stale helper was renamed in `m3_walk.rs` and `m4_walk/run.rs` (`tab_to_the_subject_and_enter`, `commit_from_the_subject`).
  - The test `the_button_commits_what_was_typed_and_then_amends_it` is now `enter_commits_what_was_typed_and_then_amends_it`. Capability M3-09's citation was updated and `gen-capabilities` rerun. The html also picked up 06b's M1 text, which had not been regenerated.
- **TESTING.md:** 41 quoted done lines are now tick-first, not just m8's lines, because they describe captures this ticket re-shot. Resolver rows and strips (`done ✓ · ▸`, `files · all done ✓`) are not done lines and stay as they are.

### Questions (not built)
- **a. PEM/ssh keys:** the spec's rule keeps a key file's last base64 line up to its `=` padding. `mask.rs:21` splits at `line.find(['=', ':'])`, and `looks_like_secrets` flags `id_rsa`, `*.pem` and `*.key` (`graft-ops/src/operation/choice.rs:70,81`). So `…IDAQAB==` stays readable up to its first `=`. Should non-`KEY=value` secrets files mask whole lines?
- **b. Resolver lines keep the tick after:** `update/resolver/resolved.rs:158` (`{} done ✓ · ▸ {}`, a notice), `view/resolver.rs:209`, `view/resolver/whole_file.rs:65-69`. None is a `Status::Done`, so 08r does not reach them. Should they follow?
- **c. m3 pty walk:**
  - `m3_walk.rs:1018` waits for `nothing staged`. Since ease-pass 08 the commit screen draws that nowhere: in `graft-ui/src` it is only `context_menu.rs:246`. The refusal words are `type a message first` / `put a file in first` (`graft-ui/src/commit_editor/words.rs:44-46`).
  - The next pty acceptance run will probably time out on `m3-05-commit-refused`. Unverified: no pty walk was run.
- **d. Stale comment:** `tests/it/m5_walk/resolving.rs:50` calls the last file's answer a `Sticky`, but `update/resolver/resolved.rs:266` builds a `Status::Done`. Only the quote was updated.

### One renderer (08r)
- `grep -rnE '\\u\{2713\}|✓' crates/graft/src crates/graft-ui/src`, tests and comments excluded, finds one tick beside a `Status::Done`: `view/status.rs:65` (`format!("✓ {what}")`).
- The rest are resolver rows and notices, the tour card, and the commit screen's `GoingIn` glyph.

### Captures
**Re-shot, done line moved, tick place only (45).** `scratchpad tick_only.py` diffed each against HEAD with the tick and `·` normalised.
- 29 are frame-only.
- 16 also carry their caption's quote turned tick-first. The two commit captions were also reworded: m5's `⏎ commit`, and commit-landed's `✓ Committed 3 files to main.  u undo · P push`, tick first.
- No other diff.

The 45 files:
- `ease-pass/`: commit-files-focus-120, commit-landed-120.
- `m6/fix-`: abort-reads-aborted-120x40; advance-{120x40,187x50}; every-file-done-footer-120x40; file-list-{80x24,120x40,160x48,187x50,200x50}; finish-shell-merge-120x40; landing-after-continue-rebase-{120x40,187x50}; last-file-opens-summary-187x50; radar-bookkeeping-120x40; shell-merge-abort-and-undo-120x40; start-over-whole-file-120x40; status-under-a-box-{80x24,120x40}; subject-enter-commits-{80x24,120x40}; three-panes-{180x50,187x50,200x50}; undo-drift-way-back-{80x24,120x40}; undo-shell-merge-{80x24,120x40}.
- `m6/`: set-aside-done-120x40, stash-clash-done-120x40.
- `m7/`: 30b-staging-not-kept-{80x24,187x50}, amend-into-landed-120x40, landed-after-merge-120x40, picker-remote-landed-120x40, pull-over-edits-120x40, put-back-{,edit-,staged-}landed-187x50, switch-landed-120x40, switch-menu-clean-landed-120x40.
- `docs/shots/`: m8-landing-undo-120x36, m8-tour-finished-120x36.
- `m8/`: came-back-landed-120x40.
- Plus `ease-pass/commit-secrets-120` (masked; Q1 commit).

**06u overlap:** `m7/landed-after-merge-120x40`, `m8-landing-undo-120x36`, and the ease-pass contact sheet. The conductor regenerates them after both merge.

**Not re-shot: old-shape done lines remain (records, not asserted; pty walks and contact sheets).**
- Pty walk records:
  - m3-01-stage-file-{80x24,160x48}, m3-02-stage-hunk-{80x24,160x48}
  - m4-08-undo-after-push-{80x24,160x48}
  - m6/m6-{01,08,11,12,17,21,24}-* at every size
  - m6/fix-band-{120x40,187x50}
- Contact sheets and caption TSVs: m3, m4, m6 (1-5 + captions), m7 (1-2 + captions), m8-contact-sheet-2 (+ captions), ease-pass-contact-sheet.
- Prototype record: m6/prototype-journeys.
- They will drift at the next acceptance run and need a `MWR_UPDATE_SHOTS=1` walk plus `scripts/contact-sheet.sh`.
- **Pty waiters:** none waits on an old-shape done line. `pty-walk-m{3,4,5,6}.py` and `m7-accept.sh` hold no `✓`. The one `on_screen` holding one, `m5_walk/resolved_operations.rs:244`, is already `✓ every file done`.

### Touches (and how each was checked)
- **Every `Status::Done` producer:** through `done_line` (grep above). Unit and it expectations were updated, and the full nextest run is green.
- **Undo consequence fitting:** consequence tests.
- **Staging-not-kept rungs:** its 6 tests.
- **Captures:** the lists above.
- **Docs:**
  - TESTING.md (m8 lines and done-line quotes) and 02-keymap :527 verbatim. The Tab cycle was checked against `graft-ui/src/commit_editor.rs:58-75` and `tab_walks_subject_body_files_and_back`.
  - Capability M3-09 and its generated html.
  - The man page has no done line, and its `Tab` row reads `Next field`, so it is unchanged. CAPABILITIES.md is unchanged.
- **Keymap:** nothing bound.
- **Journal/undo:** `done_width` only.
- **Ratchet:**
  - The `journal_put_back.rs` row went down 507→505 (rustfmt).
  - No row was raised or added.
  - `update/state.rs` is net zero (+7/−7, doc comments).
  - `view/rows.rs` is untouched.
- **MAP.md:** the mask row (Q1 commit).
- **Doc comments:** ~45 that quoted the old shape now read tick-first, each line-count neutral.

### Broken vs unverified
- **Broken (known):** the stale records above will fail a pty acceptance run until re-shot. See also question c.
- **Unverified:**
  - colour (rule 5)
  - every capture's look (rule 6, the user's eye)
  - pty walks (not run)

### Gates (part B1, workspace-wide, final tree)
- `cargo fmt --check`: clean
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo nextest run --workspace --no-fail-fast`: 6832/6832 passed, 16 skipped
- `cargo test --doc --workspace`: ok (3 passed)
