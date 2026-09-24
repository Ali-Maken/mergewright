# Review: m6-conflicts 19 (graft-ui half) + 34 (app half)

Fixed hash `3a0f31cb` (the main tree's `crates/` is identical to it at `3de12dfe`). Diff `18b11e12..3a0f31cb`, less the ticket-only and log-only commits: `2c43299c` (graft-ui widget and hints), `138b5252` (app half), `3a0f31cb` (graft-ui hint flip). Ticket 35 (choices, captures) was not reviewed.

**Counts: Critical 0 · Major 3 · Minor 9.**

## What I verified, and how

- **Gates were not run by me.** The conductor runs them. Nothing below comes from a test run. Every claim comes from reading the code at the hash.
- **Tick spot-check: 16 of 16 ticked boxes checked, 16 held.** Checked: `:155`–`:160`, `:162`–`:168`, `:173`, `:179`, `:180`. For each one I read the named test and matched its assertions against the box's words. `:161` is correctly left unticked: the editor round trip is faked, and the NOTES say so.
- **`:173` (S9, where J6 lands).** `it::finish::done_lands_on_the_merge_commit` (`tests/it/finish.rs:275-311`) runs against real git. It pumps until `in_progress` is `None` and `land_on_head` is false. Then it asserts four things:
  - `finish` is `None` and the graph has focus;
  - the selected row's oid equals `git rev-parse HEAD`;
  - `rev-list --parents` shows 3 fields (the commit and two parents);
  - the status is `Done("merged feature into main")`.
  
  No finish-only landing was added. `resumed_done` (`banner.rs:676-690`) is the old `Done` arm plus one status override, applied only for `FinishAnswer::Merged(Some(..))`.
- **S3 (no stripping on the finish route) holds.**
  - `event_loop/outside.rs:96-101` writes the message and reads it back verbatim.
  - `update/mod.rs:258-266` sends `ExternalEditor` to `finish::editor_returned` only when `finish.editing == Some(id)`.
  - `finish::editor_returned` (`finish.rs:246-270`) only splits the text.
  - `commit_editor::editor_returned`/`strip_comments` (`commit_editor.rs:377-405`) are unchanged. `the_editor_answer_for_the_commit_editor_is_unchanged` shows both routes coexisting.
  - The MAP line that said "comments already stripped" is corrected (graft MAP `:115`).
- **Hook-refusal shape, handled without swallowing.**
  - `refusal()` (`finish.rs:307-323`) matches `Conflicted { stalled: Some(words), paths }` only when `paths` is empty. That is `ApplyError::ResumeFailed` (`jobs/operate.rs:320-329`, from `resume.rs:264-271`).
  - The words become the notice. `kept_open` (`banner.rs:692-711`) still refreshes the whole scope and records the live entry.
  - This holds for both `Continue` and `ContinueWith` (see Minor 2 for the NOTES' wrong reason).
- **Keys: §2.6b plus `Ctrl+C` plus the mouse only.**
  - `finish_key` (`finish.rs:327-372`) binds `⏎`, `Ctrl+⏎`, `Ctrl+E`, `Esc`, `space`, `j`/`k` and `⇥`.
  - In the fields it also takes §2.10's editing keys, `Ctrl+←/→` included.
  - `Alt` is swallowed, `Shift+Tab`/`BackTab` are swallowed, and `Ctrl+C` returns `false`.
  - `keys_outside_2_6b_do_nothing` asserts that the whole state is unchanged.
- **Documented deviations: all accepted.**
  - `resume_answered -> Option<FinishAnswer>`: a `bool` cannot carry kept, merged and closed.
  - `from_draft`: needed so a late summary does not overwrite a restored draft (`esc_goes_back_and_keeps_the_draft` asserts this).
  - `space` works on a by-you row but is not in the bar. §2.6b `:324` says "the cursor file's diff", so the binding is within the keymap. But see Major 3.
  - `j/k move` is drawn when inert: a gap, Minor 7.
- **Clippy refactors, checked one at a time.**
  - Behaviour-neutral:
    - `runner.rs` `start_search`: the same closure, moved.
    - `mod.rs` `resized`: the same body, plus `finish::resized`.
    - `banner.rs` `resumed_done`: the old arm, plus the `Merged` override.
    - `update/tests.rs` `bar_hints`: identical for any context that is not Finish.
    - `classified_form`: the merged arms return `same.clone()`, which equals the old field-by-field rebuild.
    - `m5_walk/run.rs`: `Resolved` moved, with the same predicate.
  - Not neutral, and correct: the walk and interim changes add `Wait::FinishReady` plus `⏎` after `c` on merges (`finish_the_merge`, `committed_from_the_summary`). The feature requires this. The walk now takes `c ⏎`, 2 keys, which matches `a-finish 2` (design note `:493`). The walk count assertion is ticket 21's.
- **The feature pipeline is present.** Amendment 2 has Need (`:419-423`), Prior art rows Finish and Merge message (`:436-437`, citing `docs/research/conflict-resolution.md`) and Journeys (`:493-495`).
- **Crate rules hold.**
  - graft-ui `finish_summary` imports only graft-term and its own crate.
  - `update/` reaches `crate::view::finish`, which is established precedent, and no `ratatui` import is inside `update/`.
  - Jobs run on the pool. No git runs on the UI thread.
- **No `unwrap`/`expect` in new non-test code.** `draft.unwrap_or_default()` (`finish.rs:145`) is on an `Option` that means "no draft", not on a failure.
- **Sanitising holds.** Title, paths, `renamed from`, the notice (the hook's stderr), `Unavailable` words, diff-failure words and each body line all go through `sanitize_line`/`sanitize_path` (`view/finish.rs:24-43, 82-99, 112, 129, 147`). The raw text stays in the fields, so what lands is what was typed.

## Critical

None. 16 of 16 ticks held.

## Major

### 1. Two new files at about twice the file-size rule
`crates/graft/src/update/finish.rs` is 831 lines. `crates/graft-ui/src/finish_summary.rs` is 688 lines.
- **What's wrong:** both are new in this diff. PROMPT.md 6.2 says files stay under about 400 lines, and no `KNOWN_GAPS` size row names either file. `update/state.rs` also grew, 783 → 813; row M6/11+12 already covers that one.
- **Why it matters:** this is the readability rule, which is a hard rule. The next ticket (35) adds the choice recording to `update/finish.rs`, so it grows again.
- **Suggested fix (`codebase-design`):**
  - `update/finish.rs`:
    - `finish/answers.rs`: `summary_landed`, `diff_landed`, `job_ended`, `editor_returned`, `resume_answered`, `refusal`.
    - `finish/keys.rs`: `finish_key`, `control_key`, `files_key`, `subject_key`, `body_key`, `move_cursor`, `toggle_diff`, `commit`, `edit_outside`, `escape`.
    - `finish/mouse.rs`: `finish_mouse`, `scroll`, `click`.
    - `finish/words.rs`: `by_you_words`, `split_message`, `compose`.
  - `finish_summary.rs`: `finish_summary/layout.rs` for `ListLine`, `list_lines`, `layout`, `hit`, `max_top` and `top_for_cursor`. That is the part the app shares. The widget stays in `finish_summary.rs`.

### 2. A doc comment on the wrong function, and stacked comments on the wrong match arm
`crates/graft/src/update/banner.rs:669-675`, `:692`, and `crates/graft/src/view/frame.rs:132-142`.
- **What's wrong:**
  - `resumed_done` carries two doc paragraphs. The first, "A `--continue` the finish summary started, refused with nothing moved (a hook said no)…", describes `kept_open`, not `resumed_done`. `kept_open` (`:692`) has no doc comment at all. This is a leftover of the clippy split.
  - In `frame.rs`, three comments sit stacked over the `Finish` arm: one for the commit editor, one for the resolver, and the new one. So the `Finish` arm reads "the commit editor replaces the graph and leaves the detail pane alone…".
- **Why it matters:** PROMPT.md 6.2 (doc comments). A reader of `resumed_done` is told it handles a hook refusal, which is the opposite of what it does.
- **Suggested fix:** move `:669-672` onto `kept_open`. In `frame.rs`, put each comment over its own arm (the commit editor's over `CommitEditor`, the resolver's over `Resolver`).

### 3. A resolved file that kept yours shows a false warning when opened
`space` or a click on a by-you row whose result is `same as yours` shows `<path> is not staged` in warning style, although nothing is wrong.
- **The path through the code** (from reading; not run):
  - `space` (`update/finish.rs:410`) or a click on the row (`:694-699`) calls `toggle_diff`, which pushes `ReadFinishDiff`.
  - `jobs/finish.rs:36-43` diffs the index against `HEAD`.
  - For `ResultIs::Yours` the staged blob equals `HEAD`'s by definition (`graft-ops/src/finish.rs:225`), so there is no entry, and the job returns `Err("<path> is not staged")`.
  - That becomes `ExpandedDiff::Failed`, drawn in `Token::Warning` (`graft-ui/src/finish_summary.rs:615-616`).
- **What's wrong:** the file *is* staged; it just did not change from `main`. The words are false, and they read as an error.
- **Why it matters:**
  - CLAUDE.md: "A status field must never read as an error when nothing is wrong."
  - Taking yours for a file is a common resolution, and a click on any row toggles its diff (spec item 4, Mouse). So this is reachable on the ordinary path, not only through the unadvertised `space`.
  - The spec's `Err("<path> is not staged")` (19 item 5) assumed by-git rows only. 34's by-you extension exposed it.
- **Suggested fix:** when `list_files(Staged)` has no entry, return an empty `DiffView` rather than an error. The widget already draws `no change to show` in the muted style (`finish_summary.rs:620-621`). The job could word it `same as HEAD · no change`. Keep `Err` for real read failures. Add a unit test: a by-you row with `ResultIs::Yours` and `space` must not draw `not staged`.

## Minor

1. **`refusal()` treats every `Failed` as "nothing moved" — `update/finish.rs:307-313`.**
   - `Operated::Failed { refresh: Some(..) }` also comes from `ApplyError::Git` and `ApplyError::Journal`. `jobs/operate.rs:92-94`: "git ran **part-way**". In `resume.rs:331-337` that is `close_resume` with the merge *no longer live*.
   - The summary then stays open over a merge that has ended. Its title still says `nothing is written until ⏎`, and `⏎` answers `no merge is in progress`.
   - Fix: `Kept` only for `refused: Some(_)`, for `refresh: None`, or for the stalled `Conflicted`. Otherwise `Closed`, and let `resumed`'s `Failed` arm speak.
2. **The NOTES and the MAP give the wrong reason for the hook shape** (34 NOTES `:39`; graft MAP `:351`).
   - They say an unedited `Continue` commits through `git commit --cleanup=whitespace -F -`. `graft-ops/src/apply/resume.rs:191` shows `Resume::Continue` runs `git merge --continue`.
   - The conclusion still holds: both routes reach `still_pending`, then `ResumeFailed`, then `Conflicted { paths: [], stalled }`.
   - But the real-git hook test (`tests/it/finish.rs:245-246`) types ` (kept)`, so it covers only `ContinueWith`. The unedited `git merge --continue` hook refusal is untested against real git.
   - Fix: correct both texts. Add the unedited variant of `a_hook_refusal_keeps_the_summary_open`.
3. **Question: on an unreadable summary, `⏎` ignores what the fields show — `update/finish.rs:527`.**
   - With `Loading::Failed`, `⏎` sends `Resume::Continue`, whatever the fields hold. A restored draft or a typed subject is discarded silently, and `finish_draft` is cleared on `Done`.
   - This is exactly spec item 8, and the line says `⏎ commits with git's message`. It still departs from line 6's "the message the user sees is the message that lands".
   - Conductor: should non-empty fields send `ContinueWith(compose(..))` in this state, or should the fields be hidden?
4. **`… N more lines` counts hidden hunk headings as lines — `graft-ui/src/finish_summary.rs:220-233`.**
   - `rows` counts one heading per hunk, and `hidden = rows − drawn + cut`.
   - Example: 3 hunks of 5 lines. 12 rows are drawn (h1 + 5 + h2 + 5), and the footer says `6 more lines` where 5 are hidden.
   - Spec item 2 says N = total diff lines − lines drawn. This is a textual number drift, and the single-hunk test cannot see it.
   - Fix: subtract the headings not drawn. Add a test with several hunks.
5. **Paths with no test through `update`.**
   - Not driven through `update` by any test: `finish_mouse` (wheel; a click that moves the cursor and toggles the diff; a click that focuses the subject or body), `paste`, `resized`, and `job_ended` (a summary job that panics must not read `reading…` forever).
   - graft-ui's `a_click_lands_on_what_is_drawn` tests only `hit()`.
   - `ctrl_c_passes_through` asserts only `finish_key`'s return value, not that `Ctrl+C` still quits with the summary open.
6. **Question: the summary survives the operation ending another way — `update/undo.rs:120-131`.**
   - `operation_ended` clears `choices` and `finish_draft` but not `finish`.
   - If a re-read finds the merge gone while the summary is open (committed in the shell), the summary stays, and `⏎` says `no merge is in progress`. Not verified reachable: I did not trace whether any re-read fires while the summary is modal.
7. **No `KNOWN_GAPS` rows at the hash for:**
   - 19 `:161` (the `$EDITOR` round trip is unverified);
   - the inert `j/k move` hint (with one row, while loading, and when the read failed);
   - `space` working on by-you rows without being in the bar.
   
   Builders cannot write `KNOWN_GAPS`; this is the conductor's step. Mark the first **unverified**, the second **broken** (bar), the third a decision.
8. **Readability and docs.**
   - `update/mod.rs:265`: `let _finishes: bool = finish::editor_returned(..)`. The match guard already checked `editing == Some(id)`, so the check runs twice and the value is always `true`. Either drop the guard and branch on the return value, or make the call return `()`.
   - The graft MAP puts `jobs::finish` (`:740`) between `JobRunner::start` and `JobRunner::with_default_size`. Move it above the runner block.
   - The test name `resolver/tests.rs::all_resolved_from_the_pane_offers_c_where_c_continues` no longer describes what it asserts (the summary opens).
9. **Performance is unmeasured (unverified).**
   - Every `j`/`k`, wheel notch and click rebuilds the whole `FinishView`, sanitising every row, and walks `list_lines` at least three times (`keep_cursor_on_screen`, `update/finish.rs:725-733`, then `top_for_cursor` → `max_top` → `list_height` → `layout`, `finish_summary.rs:360-378`). The frame then builds it again.
   - A merge with thousands of files combined by git has not been measured against the 16 ms frame budget.
   - Fix: add a heavy-tier frame case with a large `by_git` before M7, or build the rows once per summary.

## Gates

Not run in this review (the conductor runs them). 34 reports fmt, clippy `-D warnings`, nextest (2940 passed, 1 skipped) and doctests all green with its own `CARGO_TARGET_DIR`. I have not observed that. It also reports that the first commit (`138b5252`) alone was not verified, and that only `3a0f31cb` passes the sweeps. The heavy tier and the release perf run are for the milestone review.
