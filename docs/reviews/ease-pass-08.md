# Review: ease-pass group 08, the commit screen

- **Scope:** ticket `.scratch/ease-pass/issues/08-commit-screen.md`, single commit f40b3538, merged at a17dbc12.
- **Range:** `git diff a17dbc12^1 a17dbc12` (identical to merge-base...^2, 125 files).
- **Mock:** `prototypes/ease-mocks/mergewright-commit-screen.html`.
- **Scouts:** 08 and 08b.
- **Reviewer:** read-only, at 4462bfe0. Skill: `code-review`. I ran both axes (standards and spec) myself, because no sub-agent tool was available.

**I did not run cargo, as I was told.** Two builders held the CPU. Every finding below comes from reading the diff, the files and the captures at 4462bfe0. I re-ran no gate, test, capture or perf number. "Gates green" is the conductor's log line (4462bfe0), not my observation.

## Verdict

| Severity | Count |
|---|---|
| Critical | 0 |
| Major | 7 |
| Minor | 12 |
| Questions | 2 |

### What holds

- **Tick spot-check: 17 of 17 ticked boxes checked, 17 held.**
  - All 43 named tests exist (grep). I read 39 of them; each asserts what its tick says.
  - Three of them assert weakly (Minor m6). None is a false tick.
  - The one unticked box (where the tick sits in the status line) is honest. 08r carries it.
- **The journey `c`, type, `⏎` is 3 steps.**
  - Real git: `tests/it/commit_screen.rs:131-170`. HEAD gets exactly README.md, docs/toc.md and src/a.rs; `.env` is left out and untouched.
  - Pure state: `screen_tests.rs` `commit_all_subject_takes_every_change_but_the_secret`.
  - `commit-with-body` 6 and `commit-leave-first` 6 match the note's fence (`docs/design-notes/ease-pass.md:62-64`).
- **`u` restores the message.**
  - `landing.rs:112-126` puts `LastCommit.message` into `commit_draft` with `amending: None`.
  - Real-git proof: `commit_screen.rs` `u_after_the_commit_gives_its_message_to_the_next_c`. HEAD goes back, `docs/toc.md` is still modified, the next `c` holds `fix toc`, and 3 files go in again.
  - Edge cases are in m1, m2 and m12. The slot it writes into is fragile (Major 1).
- **The scratch index per run (04r) is still honoured.**
  - 08 touches no `graft-ops` file (diff stat).
  - A plain commit always sends `Include::Chosen` (`commit_editor.rs:257` with `touched || !amend`), so it goes through 04r's `commit_chosen` path.
  - No new code names an index path. The new read `read_commit_files` (`jobs/kinds.rs:144-180`) is read-only `list_files`/`file_diff`.
- **No data loss on a failed commit or hook.**
  - `committed()` runs only from `landed()` (`operate/outcome.rs:137`), never on `Failed`.
  - Proof: `commit_editor::tests::a_commit_that_failed_keeps_the_editor_and_the_message` (tests.rs:876-903) and graft-ui `a_failure_takes_the_bar_row`.
  - What happens to the repository on a failed chosen commit is 04r's, and unchanged.
  - Typed words can still be lost by other paths (Major 1).
- **`commit_landed` on `done_answered` is a cause fix.**
  - `commit_landed` is new in this ticket. `done_answered` (`outcome.rs:297-315`) is where every clean write lands, so wiring it there is the cause, not a patch.
  - The third landing path, `staging_not_kept_answered` (`outcome.rs:251-277`), does not call it. That is safe today: only set-aside writes produce `StagingNotKept` (`graft-ops/src/apply/set_aside/answer.rs:195,327`), and a commit never sets edits aside.
  - The three paths each repeat the post-landing hooks, which is how the first wiring missed one (m7).
- **The boxed `AppState` fields are justified.**
  - `commit_editor: Option<Box<_>>` (`state.rs:254`) and `last_commit: Option<Box<_>>` (`state.rs:520`).
  - The builder measured 8496 → 8120 bytes against clippy's `large_stack_arrays`, which four test sites trip with `[state, state]`.
  - Boxing `last_commit` departs from the Signatures (`Option<LastCommit>`). It was needed: unboxed, the struct is about 96 bytes, which is over the 72 bytes of headroom.
  - The headroom is fragile (m9).
- **Knock-on captures: 10 of 40 re-shot captures sampled, 10 held.** Each diff contains only the named change:
  - `N commit(s) to push` at the top line's right end: `m2-force-push`, `m5-ui-confirm-over-graph-80x24`, `m6/fix-box-owns-the-bar-120x40`, `m6/fix-box-takes-y-80x24`, `m6/set-aside-preview-80x24`, `m7/put-back-landed-187x50`, `m8-tour-80x24`.
  - The subject counter `22/50`→`22/72`, with its `┊` guide moved or gone: `m6/fix-finish-summary-120x40`, `m6/fix-last-file-opens-summary-187x50`, `m6/fix-summary-preview-80x24`.
  - In `m8-tour-80x24` the repo path is cut shorter to make room for the push words. That is expected, from the new `Drop::Push` order (`graph_screen/header_tail.rs:22`).
- **Journal and undo:** journal entry words are unchanged (no `graft-ops`/journal diff). Redo and incomplete-undo edges are m1 and m2.
- **Other screens that draw the commit state:**
  - The finish summary takes `SUBJECT_LIMIT` 72 (`view/finish.rs:116`). Re-shot.
  - The tour waiters were re-pointed.
  - The m6 live-operation bar is kept through `CommitScreenView.live`. `m6/fix-editor-names-the-banner-80x24` shows the words whole.
- **Feature pipeline:**
  - The note has Need (`ease-pass.md:26`), Prior art citing `docs/research/commit-stash.md` (`:34-44`) and Journeys (`:56-64`).
  - Q9 (a) was chosen over the mock's reopen-on-`u` (`ease-pass.md:279`). The build follows (a), so that divergence from the mock is deliberate.
- **Hygiene:**
  - Sanitising: every repository string in `view/commit_editor.rs` goes through `sanitize_line`.
  - No new `unwrap()`/`expect()` outside tests. The two new `unwrap_or_default()` are on `Option`s, not failures.
  - No git on the UI thread: counts and diffs are jobs (`jobs/runner/reads.rs:273-287`).

## Critical

None.

## Major

**M1. The one `commit_draft` slot gets overwritten or cleared by unrelated events, and typed words are lost.**

- **Where:**
  - (a) `crates/graft/src/update/commit_editor/keys.rs:226`: `close()` writes `state.commit_draft = (!editor.is_empty()).then(|| editor.draft())` unconditionally.
    - Path: `c`, type `half`, `esc` keeps it; `a` (`commit_editor.rs:443-450` puts the `c` draft back), then `esc`. The amend screen is prefilled with HEAD's message, so it is non-empty, and `:226` replaces the `c` draft with an amend draft of HEAD's unchanged message. The next `c` opens empty (`commit_editor.rs:351-357`). `half` is gone.
    - Reverse path: an amend draft kept for `a`, then `c` (opens empty) and `esc`: `:226` writes `None`, and the amend draft is gone.
  - (b) `crates/graft/src/update/operate/outcome.rs:137` calls `committed()` (`keys.rs:231-234`, which sets `commit_draft = None`) on **every** write that lands, not only the screen's own commit.
    - After `esc`, or after `u` gave the message back (Q9), any stage, discard, switch or stash landing wipes the words.
    - The `landed()` comment ("The write the editor was open for landed") assumes the only write is the editor's.
    - Part (b) dates from M3 for `esc` drafts. Q9 now depends on it.
- **Why it matters:**
  - C15 and Q9 promise that the words survive. Peeking at the last commit with `a` is an ordinary move, and it silently destroys a typed message.
  - Nothing can bring it back.
  - The C15 test (`esc_keeps_subject_and_body_and_c_resumes_them_with_options_off`) covers only the direct `esc`→`c` path.
- **Fix (cause):**
  - Keep two slots, a new-commit draft and an amend draft keyed by HEAD, or never let a close overwrite a draft it did not open from.
  - Keep an amend draft only when its words differ from HEAD's message.
  - Have `landed()` clear the draft only when the landed operation is the screen's own `Commit`/`Amend` (the `launched_op` is at hand in every caller).
  - Add one test per path above.

**M2. Turning amend on from `⌃P` on a `c` screen shows "N files go in" but amends with the index as it stands.**

- **Where:** `commit_editor.rs:257`: `include_of(&self.files, self.touched || !self.amend)`. With amend toggled on and no `space` pressed, `touched` is false, so the result is `Include::Staged`.
  - `refusal()` lets it through (`:285`, `message_only`).
  - The view still counts the rows `first_for` chose at open, when amend was off (`files.rs:56`). So `view/commit_editor.rs:113,129-131` draws `3 files go in, …, into the last commit.` with the header `amending HEAD` (no `head_short`).
- **Why it matters:**
  - With nothing staged, `⏎` rewrites HEAD's message with the typed subject and puts none of the three files in. That is the opposite of what the line above the files says.
  - This breaks "you see the result before you confirm".
  - It is undoable with `u`, but the screen misled him.
  - `palette::editor_tests::choosing_each_option_twice_returns_it_to_off` toggles exactly this and asserts only the flag.
- **Fix:**
  - Key "untouched amend takes the index" on how the screen was opened (`self.amending.is_some()` / opened by `a`), not on the `amend` flag.
  - Or, when amend is toggled, re-run `files::choose` so the rows show what will go in.
  - Add a test: on a `c` screen, toggle amend from `⌃P`, press `⏎`, and assert that `Include` equals the rows going in.

**M3. A partly staged file: the counts, the summary and the diff all show HEAD→working tree, but only the staged part goes in.**

- **Where:**
  - When anything is staged, `first_choice` gives `Part::StagedPart` (`graft-ops/src/operation/choice.rs:46-48`).
  - The diff is always read with `section: None`, which is working tree vs HEAD (`update/commit_editor/files.rs:196-203`).
  - The counts come from `DiffSpec::WorkingTreeVsHead` (`jobs/kinds.rs:151`).
  - The summary adds those counts up (`view/commit_editor.rs:118-127`).
- **Why it matters:**
  - The `+a −d` he agrees to, and the lines he reads in the diff, include unstaged lines that will not be committed.
  - The builder filed this as "unverified" (NOTES). By the lines above it is **broken**, not unverified.
  - Same WYSIWYG promise as M2.
- **Fix:**
  - For a `StagedPart` choice, read the diff and counts with `section: Some(Staged)` (index vs HEAD).
  - Or show both and label the part that stays out.
  - Add a state test with one partly staged file asserting that the `DiffKey.section` is the staged one.

**M4. The landing never shows `JustCommitted`'s bar. The mock shows the done line and the bar together.**

- **Where:**
  - Mock `:178-179`: the done line *and* the bar `u undo the commit, your edits come back · P push · c commit again`.
  - Capture `docs/shots/ease-pass/commit-landed-120.txt`, last row: only `Committed 3 files to main. ✓ · u undo · P push`. The caption itself says the bar is "under it, not drawn while the status stands".
- **Why it matters:**
  - The `JustCommitted` words exist to say that `u` gives the edits back. At the moment they matter most, they are hidden.
  - They appear only after a key clears the status and the cursor is back on the commit.
  - The C13 tick is honest ("checked by state, not by drawing"), but the user gets something other than the mock he accepted (memory `build-to-the-mocks`).
- **Fix:** a conductor decision.
  - Either the graph draws the status line above the bar for a `Done` that has a landing bar, as the mock does,
  - or the mock's two-line landing goes back to the user before 08r re-shoots the done lines.

**M5. The mock's click on a file row is not built, and clicks on the commit screen reach the hidden graph.**

- **Where:**
  - Mock `:146`: a click on a file row moves the file cursor.
  - `update/mouse.rs:21-99` has no arm for the commit screen. `left_click` → `graph_row_at` (`:163-170`) selects a graph row behind the screen.
  - `right_click` (`:134-136`) opens a row's context menu for a row he cannot see.
  - The comment at `:31-37` admits this for the Map, journal and stash overlays. The commit screen now covers the whole frame, the detail pane included.
- **Why it matters:**
  - A mock behaviour is an acceptance line (conductor, `build-to-the-mocks`), and the ticket did not carry this one. That is a spec gap, not the builder's.
  - The file list invites clicks. Today a click moves a cursor he can't see.
- **Fix:**
  - Add a `commit_editor` mouse arm before `palette_mouse`'s fall-through: a click on a file row picks it, and every other event is swallowed.
  - Or record a gap row and take it back to the user.

**M6. Modularity: `update/commit_editor.rs` crossed 400 lines, and nine production files had their ceilings raised (standing rule, 2026-10-18).**

- **Where:** `crates/graft/tests/it/over_400.txt`.
  - New row: `crates/graft/src/update/commit_editor.rs` 399 → **489**. This is the ticket's own module, and it already has six submodules.
  - Raised ceilings:

    | File | Before | After |
    |---|---|---|
    | `update/operate/outcome.rs` | 415 | 430 |
    | `update/events.rs` | 598 | 603 |
    | `update/events/command.rs` | 484 | 487 |
    | `update/state.rs` | 1312 | 1319 |
    | `view/frame/bars.rs` | 413 | 418 |
    | `view/frame/boxes.rs` | 460 | 462 |
    | `view/rows.rs` | 894 | 897 |
    | graft-ui `graph_screen/layout.rs` | 650 | 653 |
    | graft-ui `graph_screen/mod.rs` | 573 | 578 |

- **Why it matters:**
  - PROMPT.md 6.2. Review 06 M6 set the precedent that raising the ratchet is a finding.
  - Most of these are one-line appends that were forced. `commit_editor.rs` was not forced.
- **Fix:**
  - Move `CommitOption` and its `impl` (`commit_editor.rs:46-97`) into `commit_editor/options.rs`.
  - Move `open`, `show` and `head_message_arrived` (`:330-461`) into `commit_editor/open.rs`.
  - That leaves the module root under 400. Record the forced appends as accepted, by name.

**M7. Readability: `head_message_arrived` is 73 lines with five levels of nesting (PROMPT 6.2).**

- **Where:** `crates/graft/src/update/commit_editor.rs:388-461`. The deepest point is `:443-450`: `match` → `Ok` → `let … = if let … else` → `if let Some(draft)` → `if draft.amending…`. This ticket added the draft-resume block.
- **Why it matters:** it is the function M1(a) runs through, and the place where the draft swap hides.
- **Fix:**
  - Extract `fn amend_screen(state, head, message, reword_line) -> CommitEditor`.
  - Extract `fn resume_amend_draft(state, &mut editor, head)`.
  - Leave `head_message_arrived` as the guard-and-dispatch.

## Minor

**m1. A redo after `u` leaves the returned message as the draft.**
- `undo/answers.rs:171-172`: a redo clears `last_commit` but not the `commit_draft` that `undo_landed` set.
- So `u`, `U`, `c` opens prefilled with the subject of a commit that exists again, which invites a duplicate commit.
- **Fix:** on a redo that re-creates the commit, clear a draft equal to that commit's message.

**m2. An incomplete undo of the commit neither gives the message back nor clears `last_commit`.**
- `undo/answers.rs:212-217`: the `Undone::Incomplete` arm only refreshes.
- `last_commit` still points at the commit that was taken back.
- **Fix:** run the same `undo_landed`/redo branch as `undid`.

**m3. Counts and diff go stale while the screen is open.**
- `files.rs:105` returns early when the set of files is unchanged.
- An edit to a listed file (the watcher re-reads) leaves the `+a −d` and the diff as they were. The commit takes the new contents.
- **Fix:** compare the read's per-file stamps, or always re-ask the counts and the cursor file's diff on a re-read.

**m4. Narrow bar: the wrong part drops.**
- `commit-90.txt`, last row: under 100 columns the changes pane is hidden, yet the bar keeps `↑/↓ see a file's changes` (`words.rs:83`).
- It drops `esc cancel, your message is kept` (`words.rs:84`, cut by `whole_parts`, draw.rs:103), the one hint that says leaving is safe.
- He lives in Zed and VS Code panes (memory), so narrow widths are his daily view.
- **Fix:** omit the `↑/↓` part when the diff is hidden, and drop parts by priority, not from the right.

**m5. A caption disagrees with its picture.**
- `docs/shots/ease-pass/commit-files-focus-120.txt:4` says the bar reads `⏎ commit 3 files`.
- The capture's last row reads `⏎ commit`. That matches the mock (`:162`) and `words.rs` `enter_words(view, false)`.
- **Fix:** correct the caption.

**m6. Weak asserts behind true ticks.**
- `screen_tests.rs:360` `undoing_something_else_leaves_the_draft_alone` starts with `commit_draft = None` and asserts `None`. A clearing bug would pass it. **Fix:** seed `Some(draft)`.
- `screen_tests.rs:327` `another_write_landing_clears_the_last_commit` calls `commit_landed` with an amend. It never lands a non-commit write through `landed()`. **Fix:** drive a stage `Operated` through `update`.
- The journey counters are tautological: `commit_screen.rs:144-149` (`keys += 1` three times, then `assert_eq!(keys, 3)`), and `steps.len()` on array literals in `screen_tests.rs`. The journeys still prove themselves, because the commit is asserted after exactly those inputs. But the "fails one key over" check the note's fence promises (`ease-pass.md:58`) is A's walk, not this.

**m7. `incomplete_answered` builds the commit's done line and then throws it away.**
- `outcome.rs:34-38` sets `state.status` from `commit_landed`, and `:42` overwrites it with `warning`.
- The three landing paths (`:33-42`, `:251-277`, `:297-315`) each repeat `landed` + `amend_into::landed` + `recover::tag_rows`, which is how the first wiring missed `done_answered`.
- **Fix:** one `after_landing(state, launched_op, …)` that all three call. Discard the words explicitly in the incomplete path.

**m8. Stale docs.**
- TESTING.md:1420, 1473, 1478, 1482 still walk through `Tab` to `[ Commit ]`.
- `docs/design/02-keymap.md:527` `Tab` row says "subject / body / options". It is files now (`:534` already says the options row is gone).
- The helper `press_the_button` (`tests/it/commit_editor_writes.rs:178`) now tabs to the subject.
- **Fix:** reword all three. The keymap row needs the conductor (Out of scope for 08).

**m9. `AppState` has 72 bytes of headroom under `large_stack_arrays`** (builder NOTES; `state.rs:254,520`).
- The next field over 72 bytes breaks clippy at four test sites again.
- **Fix:** turn the four `[state, state]` sites into maker closures (the precedent is `view/tests.rs:4786`), so boxing stops being the price of every new field.

**m10. Superseded line-count reads keep running.**
- `files.rs:114` / `commit_editor.rs:380` set a new `counts_job` without cancelling the old one.
- Each read diffs up to 50 untracked files whole (`jobs/kinds.rs:135,157`). A watcher burst while the screen is open queues them back to back.
- Also, one unreadable untracked file fails the whole read (`kinds.rs:167` `?`), so every row loses its counts.
- **Fix:** cancel the previous job. Skip a single file's failure, leaving its counts `None`.

**m11. `undo_landed`'s "took it back" test is loose.**
- `landing.rs:116-119` matches any `refs_before` value equal to the commit. It does not check that the ref moved off it.
- Safe while `last_commit` is cleared by every other landing.
- **Fix:** `refs_before[r] == Some(c) && refs_after[r] != Some(c)`.

**m12. The builder's unverified items are not in KNOWN_GAPS.**
- No counts on an unborn HEAD, and colour and look (rules 5 and 6).
- Also, the screen header drops the mock's `mwr  ~/work/reports` (mock `:83`; `commit-open-120.txt` line 1 shows only `committing to main`). He works across several repositories in a team.
- **Fix:** the conductor adds the rows. Decide whether the header names the repository.

## Questions (rule 7: no line proves a defect)

**Q1. Should a secrets-looking file's diff mask its values?**
- The mock shows `.env.local` as `PAYMENT_KEY=••••••••` (mock `:116`).
- The build prints the raw value: `commit-secrets-120.txt` shows `1 + TOKEN=x`.
- This may be only how the mock was drawn. If masking is meant, it is unbuilt. Either way, captures of real secrets would be committed to the repo.

**Q2. What does a drag or right-click on the commit screen do to the hidden graph?**
- `update/mouse.rs:84` (`drag_mouse`) and `:134-136` (`context_menu::open_for_history_row`) run with the screen up.
- A menu's letter goes back down `key_event`, which the screen claims (`update/keys.rs:320`), so it would be typed into the subject.
- I did not trace whether a badge drop opens a preview over the screen. Folds into M5's fix.

## Gates

I did not run them (conductor's instruction). Not observed by me: fmt, clippy, nextest, doc, the heavy tier, the release perf run, test times, peak memory and `target/` size.
