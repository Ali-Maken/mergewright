# fix-finish-summary-screen: `c` on a merge shows what will land — what you resolved, what git combined on its own, the message — and `⏎` commits it

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 09-ops-conflict-set-and-summary.md, 10-resolver-line-numbers.md, 22-ops-finish-message-and-step-bound.md, 13-resolver-file-list.md, **16-take-every-open-block.md** (`TakeAt`, `apply_take_all`), **17-resolver-file-keys.md** (`O`/`T` land through the same `take_landed`; same crate set, ADR-0036: not concurrent)
Was: draft · new 2026-09-23. Part A approved (field notes 2026-09-23: *"yes build it that way"*).
**Revised 2026-09-23 (review `m6-amend-09` Minor 1).** The message the user sees is the message that lands, character for character. git's comment lines come out when `MERGE_MSG` is **read** (`finish_summary`). The commit keeps what he typed (`ContinueWith` sends `--cleanup=whitespace`, graft-ops, ticket 22). A `#123 fixes login` line he types survives.
**Revised 2026-09-24 (spec-only pass on `.scratch/m6-conflicts/scout-19.md`, main `4ec8fd73`).** Rulings S1–S9:
- **S1 (scout 1).** `TakeAt` is ticket 16's; this ticket builds after 16 and 17. Nothing here is designed around its absence.
- **S2 (scout 2).** `Choices.takes` holds `graft_git::conflict::Take` (Ours/Theirs/Both, `graft-git/src/conflict.rs:208`), the type `TakeRequest.take` carries. `Choices.whole` holds `graft_ops::operation::conflict::Side` (Ours/Theirs, graft-ops MAP §operation `Side`), the whole-file write's side.
- **S3 (scout 3).** `Ctrl+E` gets its **own return route**. The handover writes the message as-is, no comment lines (`event_loop/outside.rs:96`), so there is nothing of ours to strip. `commit_editor::editor_returned` strips every `#` line (`commit_editor.rs:399-405`); the finish never goes through it.
- **S4 (scout 4).** The diff under a row gets its **own command and product**. `ReadFileDiff` needs the detail pane's `DiffKey` and its `Product::FileDiff` always goes to the detail pane (`update/mod.rs:194-200`). The new job takes git's own `FileEntry` from `list_files(Staged)` rather than fabricating one from `ByGit` (a rename's similarity is not in `ByGit`).
- **S5 (scout 5).** The finish widget passes `truncated: None` to `diff_body`, so `L load all` never draws (`L` is not a §2.6b key), and draws its own footer `… N more lines · space hides`.
- **S6 (scout 6).** Keys are §2.6b's and nothing else, plus `Ctrl+C` (§2.1 modal rule) and the mouse (§2.15). `⇧⇥`, `↑`/`↓` in the file list and `PgUp`/`PgDn` are **not bound**: §2.6b does not list them. Proposed as P11 under "For the conductor". `Esc` goes back to where `c` was pressed (design note :478); §2.6b:323 says "the resolver" only, which a banner-opened summary cannot honour. Correction text below; the conductor writes it before the binding lands.
- **S7 (scout 7).** `finish_draft` stays `(subject, body)`, apart from `commit_draft`. The commit editor's draft carries amend/sign-off/no-verify, which a merge commit has none of, and a draft typed before the merge must survive it. Stored only when it differs from git's message.
- **S8 (scout 8).** Part B (`c` in the resolver, the resolver staying open, the notice-row words) moves to its own spec, `finish-from-the-resolver.md`, which builds after this one. P8 is approved (`02-keymap.md:269`, `:275`), so that spec isn't gated.
- **S9 (could not confirm).** The scout did not trace where `resume`'s `Done` puts the graph cursor. This ticket adds **no landing of its own**. `J6`'s landing is an acceptance box whose test observes it against real git. If it fails, stop and report the line where the landing goes elsewhere. Don't patch a finish-only landing.

Build with: **Opus** (scouted: `scout-19.md`). A new modal with two text fields, two new jobs, and answer routing that has to cut in ahead of `banner::resumed` and `commit_editor::editor_returned` without changing either for their own callers. Not Fable: every shape is fixed here, nothing later hangs on a choice the builder makes, and the tests judge it.
Needs: 09 (`finish_summary`, `FinishSummary`, `ResultIs`, `ByGit`, `SummaryError`); 22 (`Resume::ContinueWith`, `--cleanup=whitespace`); 10 (`SideNames`, `side_names`); 16 (`TakeAt`, `TakeRequest.at`, `take_landed` reading it); 17 (`O`/`T` through `take_landed`); graft-git `diff::list_files`, `diff::file_diff`, `DiffSpec::Staged`; graft-ui `detail_pane::diff_body`, `text_input::{TextField, TextArea, FieldView, AreaView}`.
Crates: graft-ui, graft.

## Read first
- This ticket, and `.scratch/m6-conflicts/scout-19.md` (whole, one page).
- `docs/design-notes/m6-conflicts.md` Amendment 2: Need (his words on finishing, :419-423), Prior art rows Finish and Merge message (:436-437), Journeys J6, J6b, J10 (:458-462), "Where each event lands him" (:467-481), Hard choice 3 (:610), New UI "Finish summary" (:707-747). **Need, Prior art and Journeys are unchanged by this revision**; counts stay `a-finish 2`, `b-finish 2`, `summary-open-git-combined 1`.
- `docs/design/02-keymap.md` §2.1 :58-81 (modal panes), §2.6a :291-299, §2.6b :312-327, §2.10 :361-370, §2.15 :416-420.
- `crates/graft/MAP.md` §`update::banner` (`continue_pressed`, `continue_refusal`, `resume`, `resumed`), §`update::commit_editor` (`edit_outside`, `editor_returned`), §update (`AppState::text_field_open`, `modal_owns_keys`, `graph_overlay`, `GraphOverlay`, `external_editor`, `land_on_head`, `Command`, `Product`), §`update::resolver` (`take_landed` as 16/17 leave it, `restart_landed`), §`update::undo` (`operation_ended`).
- `crates/graft-ui/MAP.md` `text_input`, `commit_editor` (the look to copy), `detail_pane` (`diff_body`, `DiffFocus`), `DiffView`, `graph_screen::hints` (`HintContext`, `every_context`).
- `crates/graft-git/MAP.md` §diff (`list_files`, `file_diff`, `DiffSpec::Staged`, `Truncated`).

## Scope
**In.**
1. **Open.** In `banner::continue_pressed`, the owed-read and `continue_refusal` checks stay as they are (`banner.rs:395-416`). Only the last step changes (`:411`). For a merge, call `open_finish(state, commands, ReturnTo::Banner, next_id)`; rebase, cherry-pick and revert keep `resume(.., Resume::Continue, ..)` (gap row → M7).
   - `open_finish` sets `AppState::finish`, puts `finish_draft` into the fields if there is one, focuses `Files`, and pushes `Command::ReadFinishSummary(id)`. The job runs `graft_ops::finish::finish_summary` (`jobs/finish.rs`, new).
   - When the summary lands, and only if no draft was restored, the fields are filled from `FinishSummary::message`: the first line is the subject, and the rest (after one blank line, if there is one) is the body. The cursor opens on the first `by_git` row, else the first `by_you` row.
2. **Draw** (`graft-ui/src/finish_summary.rs`, new), per the note's mockup (:708-723):
   - Title `Finish · merging <from> → <into> · nothing is written until ⏎`.
   - `RESOLVED BY YOU · n`, rows worded by `by_you_words` (item 3).
   - `COMBINED BY GIT ON ITS OWN · n · space shows the change`, rows `path  <change>  +a −r`. `Change` words: `added`, `modified`, `deleted`, `renamed from <from>`, `type changed`. With nothing combined, `COMBINED BY GIT ON ITS OWN · none`.
   - `MESSAGE · ⇥ edit`, then the subject field and the body area, in §2.10's look.
   - Below `TWO_COLUMN_MIN` (160), one column. At 160 and wider, the file lists take the left 60 % and the message the right.
   - The cursor row is `▸`. An expanded diff draws under its row, indented 4, at most 12 rows, through `diff_body` with `truncated: None` (S5). When more rows exist, it adds one row `… N more lines · space hides`, where N = total diff lines − lines drawn, and total = `DiffView.truncated.total_lines` when set, else the lines in its hunks.
   - `notice` draws above the bar (problem style when its flag is `true`).
3. **`Choices` and their words.** `AppState::choices: BTreeMap<PathBuf, Choices>`.
   - **Written:**
     - `take_landed`, for every take answer, queued ones included. `TakeAt::Block(i)` sets `takes[i]`. `TakeAt::Every` sets `takes[i]` for every block `i` that the request's document had open and the landed document took. `takes` is sized to the document's `blocks().len()` on first write.
     - Every `ResolveWholeFile` answer that landed sets `whole = Some(side)`.
     - Every `$EDITOR` or mergetool return for that path (resolver `e`/`M`, banner `e`/`M`) sets `edited = true`.
   - **Removed:** `restart_landed` (`d`, start over) removes that path's entry, because the file is conflicted afresh.
   - **Cleared:** the whole map, and `finish_draft` too, in `undo::operation_ended` and on an abort's answer.
   - **Words** (`by_you_words`), first match wins:
     1. `whole = Some(s)` and `ResultIs::Deleted` → `deleted · as <role · name of s>`.
     2. `whole = Some(s)` → `kept · <role · name of s>`.
     3. `edited` → `edited in your editor`.
     4. Some takes recorded, and all of one kind → `<role · name> in k of n conflicts` (`both` for `Both`), where n = `takes.len()`. Mixed kinds → `yours · main in 1, theirs · feature in 1, both in 1`, in the order yours, theirs, both, each only if non-zero.
     5. No entry → `ResultIs`: `same as yours · main`, `same as theirs · feature`, `deleted`, `combined`.

     `role · name` comes from `side_names` (ticket 10), the same words the resolver uses.
4. **Keys.** `finish_key` is modal. It runs after the drag and box checks and **before** `resolver_key` (`keys.rs:95-98`). `modal_owns_keys` includes `finish.is_some()`, and `text_field_open` includes it while focus is `Subject` or `Body`. `GraphOverlay::Finish` comes first in precedence (`state.rs:498`).

   | Focus | Key | Does | Authority |
   |---|---|---|---|
   | files | `j` / `k` | move the cursor, kept on screen | §2.6b :325 |
   | files | `space` | expand/collapse that row's diff (a `by_git` row, or a `by_you` row) | §2.6b :324 |
   | any | `⇥` | files → subject → body → files | §2.6b :326 |
   | files, subject | `⏎` | commit | §2.6b :321 |
   | body | `⏎` | newline | §2.6b :321 |
   | any | `Ctrl+Enter` | commit | §2.6b :322 |
   | any | `Ctrl+E` | `$EDITOR` on the whole message (item 6) | §2.6b :327 |
   | any | `Esc` | close; the draft goes to `finish_draft`; `ReturnTo::Banner` → the banner, focused | §2.6b :323, corrected (S6) |
   | subject, body | printable keys and §2.10's editing keys | edit, as the commit editor's fields do | §2.10 |
   | any | `Ctrl+C` | passes through (quit) | §2.1 |
   | any | anything else | swallowed | §2.1 |

   Mouse (§2.15): a click on a row moves the cursor there and toggles its diff; a click in the subject or body focuses it; the wheel scrolls the summary.
5. **Diff under a row.** `space` on a row with no diff shown pushes `Command::ReadFinishDiff(id, path)` and sets `expanded = Some((row, Pending))`; `space` again collapses it. Only one row is expanded at a time, and a stale answer (id ≠ `diff_job`) is dropped; comment why that is safe.
   The job runs `graft_git::diff::list_files(.., DiffSpec::Staged, ..)`, takes the entry whose `path` matches, then runs `file_diff` with `DiffSpec::Staged`, `DiffOptions::default()` and the detail pane's line cap. It converts to `DiffView` exactly as the detail job does (reuse it; don't copy it). No entry → `Err("<path> is not staged")`, shown in place of the diff.
6. **Commit and `$EDITOR`.**
   - `compose(subject, body)`: the subject, plus `"\n\n" + body` when the body isn't blank.
   - The message counts as unchanged when `compose(..).trim_end()` equals `FinishSummary::message`, lossily decoded, `.trim_end()`. Unchanged → `Resume::Continue`. Changed → `Resume::ContinueWith { message: compose(..).into_bytes() }`. The app strips nothing.
   - `Ctrl+E` pushes the same `Command::EditCommitMessage { id, message: compose(..) }` that `edit_outside` sends (`commit_editor.rs:519`), sets the handover state `edit_outside` sets, and sets `FinishState::editing = Some(id)`.
   - In `update/mod.rs` (`:427`), `Product::ExternalEditor` goes to `finish::editor_returned` when `finish.editing == Some(id)`, before `commit_editor::editor_returned`. `finish::editor_returned` splits the text as in item 1 and **strips nothing** (S3).
   - graft MAP line 115 says the product carries "comments already stripped", but the scout found the strip in `commit_editor.rs:399`. Confirm which is true in NOTES. If anything strips before the product, the finish route must not pass through it, and `a_hash_line_from_the_editor_lands` is what proves that. Correct the MAP line.
   - Empty subject → notice `the message is empty · ⇥ to write one`, no write.
   - `⏎` or `Ctrl+Enter` sets `committing = Some(id)` via `resume` (`banner.rs:478`). While `committing`, the bar reads `committing…` and every key but `Ctrl+C` is swallowed.
7. **The answer.** `banner::resumed` first calls `finish::resume_answered(state, id, ..)`, which acts only when `finish.committing == Some(id)`.
   - `Done` → close the summary, clear `finish_draft`, focus the graph, then return control to `resumed`'s own `Done` handling (landing, `operation_ended`) **unchanged**. After it, set the status to `merged <from> into <into> ✓`.
   - A failure (a hook) → the summary stays open, `committing = None`, the reason becomes `notice` (problem), and the draft is kept.
   - `Conflicted` or any other answer → close the summary as on `Done`, keeping the draft in `finish_draft`, and let `resumed` handle it as today.
8. **States.**
   - Loading → body `reading what this merge changes…`; `⏎` → notice `still reading · wait`.
   - `StillUnmerged` or another read failure → `summary unavailable: <why> · ⏎ commits with git's message`; `⏎` → `Resume::Continue`.
   - `recorded: false` → one section, `CHANGED BY THIS MERGE · n · which were conflicts is not recorded`.
   - Frame < `FINISH_MIN` (60×14) → one line `finish needs 60x14 · esc back`; only `Esc` (and `Ctrl+C`) act.
9. **Hints.** `HintContext::Finish { focus, expandable }`:
   - files: `⏎ commit  space diff  j/k move  ⇥ message  esc back` (`space diff` only when `expandable`);
   - subject: `⏎ commit  ⇥ body  ^E editor  esc back`;
   - body: `^⏎ commit  ⇥ files  ^E editor  esc back`.

   It joins `every_context` (`hints/catalogue.rs:34`), `coverage_slot` (`graph_screen/tests.rs:1945`) and `BOUND_ACTIONS` (`keys.rs:755`).
10. MAP.md: graft (§update: `finish`, the new fields, commands and products; §`update::finish`; §jobs `finish`; the MAP line 115 fix) and graft-ui (`finish_summary`, the hint context).

**Out.**
- `c` in the resolver, the resolver staying open, the notice-row words (`finish-from-the-resolver.md`).
- Rebase, cherry-pick and revert summaries (M7).
- P4's `FinishMerge` (ticket 20 routes its `c` through `open_finish`).
- Horizontal scroll in the diff.
- The cleanup mode (ticket 22).
- `⇧⇥`, `↑`/`↓` in the list, `PgUp`/`PgDn` (P11, not approved).

## Signatures
```rust
// graft-ui/src/finish_summary.rs (new)
pub struct SummaryRow { pub path: SanitizedText, pub what: SanitizedText }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum SummaryFocus { Files, Subject, Body }
pub struct FinishView<'a> { pub title: SanitizedText, pub by_you: Vec<SummaryRow>, pub by_git: Vec<SummaryRow>, pub recorded: bool, pub cursor: usize,
    pub expanded: Option<(usize, ExpandedDiff)>, pub subject: FieldView<'a>, pub body: AreaView<'a>, pub focus: SummaryFocus,
    pub notice: Option<(SanitizedText, bool)>, pub busy: bool, pub state: SummaryState, pub top: usize }
pub enum ExpandedDiff { Loading, Failed(SanitizedText), Ready(Arc<DiffView>) }
pub enum SummaryState { Loading, Ready, Unavailable(SanitizedText) }
pub struct FinishScreen<'a> { pub view: &'a FinishView<'a>, pub theme: &'a Theme, pub ascii: bool }   // Widget
pub const FINISH_MIN: (u16, u16) = (60, 14);
pub const TWO_COLUMN_MIN: u16 = 160;
pub const DIFF_ROWS_MAX: usize = 12;
pub enum HintContext { /* existing */ Finish { focus: SummaryFocus, expandable: bool } }

// graft/src/update/finish.rs (new)
pub struct FinishState { pub summary: Loading<Arc<FinishSummary>>, pub job: Option<JobId>, pub cursor: usize, pub top: usize,
    pub expanded: Option<(usize, Loading<Arc<DiffView>>)>, pub diff_job: Option<JobId>,
    pub subject: TextField, pub body: TextArea, pub focus: FinishFocus, pub return_to: ReturnTo,
    pub committing: Option<JobId>, pub editing: Option<JobId>, pub notice: Option<(String, bool)> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum FinishFocus { Files, Subject, Body }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum ReturnTo { Banner, Resolver }   // Resolver: used by finish-from-the-resolver
pub fn open_finish(state: &mut AppState, commands: &mut Vec<Command>, from: ReturnTo, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn finish_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress, next_id: &mut impl FnMut() -> JobId) -> bool;
pub(crate) fn summary_landed(state: &mut AppState, id: JobId, summary: Result<Arc<FinishSummary>, String>);
pub(crate) fn diff_landed(state: &mut AppState, id: JobId, diff: Result<Arc<DiffView>, String>);
pub(crate) fn editor_returned(state: &mut AppState, id: JobId, text: Result<String, String>) -> bool;  // true: it was the finish's
pub(crate) fn resume_answered(state: &mut AppState, id: JobId, /* resumed's answer type, by reference */) -> bool;
pub(crate) fn compose(subject: &str, body: &str) -> String;
pub(crate) fn by_you_words(choices: Option<&Choices>, result: ResultIs, names: &SideNames) -> String;
pub struct Choices { pub takes: Vec<Option<graft_git::conflict::Take>>, pub whole: Option<graft_ops::operation::conflict::Side>, pub edited: bool }

// graft/src/update/state.rs
pub struct AppState { /* existing */ pub choices: BTreeMap<PathBuf, Choices>, pub finish: Option<FinishState>, pub finish_draft: Option<(String, String)> }
pub enum GraphOverlay { Finish, /* existing */ }   // first in precedence

// graft/src/update/events.rs
pub enum Command { /* existing */ ReadFinishSummary(JobId), ReadFinishDiff(JobId, PathBuf) }
pub enum Product { /* existing */ FinishSummary(Result<Arc<FinishSummary>, String>), FinishDiff(Result<Arc<DiffView>, String>) }

// graft/src/jobs/finish.rs (new): the two jobs of items 1 and 5.
```

## Acceptance (name the verifying test when you tick; truth from git)
- [ ] `update::finish::tests::c_on_a_resolved_merge_opens_the_summary_not_the_commit`: no `Resume` command, `finish` is `Some`, `ReadFinishSummary` issued.
- [ ] `…::c_with_a_file_unmerged_is_still_refused`: `continue_refusal`'s words, no `finish`.
- [ ] `…::a_rebase_c_still_continues_directly`: `Resume::Continue`, `finish` is `None`.
- [ ] `…::enter_commits_with_gits_message_when_unedited`: `Resume::Continue`.
- [ ] `…::an_edited_subject_commits_with_that_message`, walk harness, real git: `ContinueWith`; `git log -1 --format=%B` reads back the typed subject.
- [ ] `…::a_hash_line_typed_in_the_body_lands`, real git: `⇥ ⇥`, type `#123 fixes login`, `Ctrl+Enter` → `%B` contains the line `#123 fixes login`.
- [ ] `…::a_hash_line_from_the_editor_lands`, real git: `Ctrl+E` with a fake `$EDITOR` that appends `#123 fixes login` → the body holds that line; after `Ctrl+Enter`, `%B` contains it (S3).
- [ ] `…::the_editor_answer_for_the_commit_editor_is_unchanged`: an `ExternalEditor` answer under the commit editor's id still reaches `commit_editor::editor_returned`, and the finish is untouched.
- [ ] `…::the_summary_shows_no_git_comment_lines`: the fixture's `MERGE_MSG` has git's `# Conflicts:` block → no drawn message line starts with `#`.
- [ ] `…::esc_goes_back_and_keeps_the_draft`: `⇥`, type, `Esc` → the banner is focused; `c` → the typed subject is back.
- [ ] `…::esc_with_nothing_typed_keeps_no_draft`: `Esc` → `finish_draft` is `None`.
- [ ] `…::space_expands_a_git_combined_file`: one `ReadFinishDiff` for `README.md`; `space` again collapses; a stale answer is dropped.
- [ ] `jobs::finish::tests::the_diff_is_the_staged_change`, real git: `README.md` → one hunk, whose removed and added lines equal `git diff --cached README.md`'s.
- [ ] `update::finish::tests::a_hook_refusal_keeps_the_summary_open`: a failing `commit-msg` hook → `finish` is `Some`, the notice holds the hook's words, and the draft is kept.
- [ ] `…::the_summary_words_what_you_chose`: `Ctrl+T ⏎` on `notes.txt`, `o` on `gone.txt` → `theirs · feature in 2 of 2 conflicts`, `kept · yours · main`. After a relaunch (no `choices`), the rows read `ResultIs` words.
- [ ] `…::shift_t_records_its_choices`: `T` on `notes.txt` → `theirs · feature in 2 of 2 conflicts` (ticket 17's `O`/`T` through `take_landed`).
- [ ] `…::a_start_over_forgets_the_choices`: `t`, then `d` confirmed → no `choices` entry for the file.
- [ ] `…::choices_and_draft_clear_when_the_merge_ends`: after `Done`, and after an abort, `choices` is empty and `finish_draft` is `None`.
- [ ] `…::done_lands_on_the_merge_commit`, walk harness, real git (S9, J6's landing):
  - after `⏎`, with every read the landing owes answered, `finish` is `None`, the graph is focused, and the cursor row's oid equals `git rev-parse HEAD`;
  - `git rev-list --parents -1 HEAD` shows two parents;
  - the status reads `merged feature into main ✓`.

  If it fails, report the line where the cursor goes elsewhere. Add no finish-only landing.
- [ ] `…::keys_outside_2_6b_do_nothing`: `↑`, `PgDn`, `⇧⇥` in files focus → no state change, no command (S6).
- [ ] graft-ui `finish_summary::tests::{one_column_below_160, two_columns_from_160, too_small_says_so, a_long_diff_stops_at_12_rows_and_says_space_hides}`. The last one: no `L load all` anywhere in the buffer. Plus `hints::tests::every_context_reaches_every_shape`, and `update::tests::every_bound_action_is_hinted_live_or_is_named_here`, green.
- [ ] Journeys `a-finish` 2 and `summary-open-git-combined` 1 hold in the walk (ticket 21 counts them).
- [ ] Captures `docs/shots/m6/fix-finish-summary-{120x40,160x48,200x50}.txt` and `…-expanded-120x40.txt`, pinned by `it::finish::summary_captures`. The fixture is a two-file merge where `feature` also changes one line of `README.md` cleanly. **What right looks like:**
  - The title reads `Finish · merging feature → main · nothing is written until ⏎`.
  - `RESOLVED BY YOU · 2` lists `gone.txt` and `notes.txt` with the words above.
  - `COMBINED BY GIT ON ITS OWN · 1` lists `▸ README.md  modified  +1 −1`.
  - The message's first line is `head -1 .git/MERGE_MSG` from the fixture, quoted in the TESTING line. Don't assume the wording.
  - No message line starts with `#`.
  - The bar starts `⏎ commit`.
  - At 160 and 200, the message sits to the right of the lists.
  - Expanded: the diff rows sit under `README.md`, indented, and the rows after them move down. No `L load all`.
  - The user judges by eye (rule 6). Colour is unverified (rule 5).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium–large, one context: one widget, one modal state machine, two jobs, three answer routes, ~22 tests, four captures. If it won't fit, split at the seam: graft-ui widget and hints first (with their tests and a fixture-built `FinishView`), the app second. Say so; don't compress.

## For the conductor (outside this ticket; exact text)
1. **`docs/design/02-keymap.md:323`** (a wording correction to an approved row, not a new key). Replace it with:
   `| \`Esc\` | Back to where \`c\` was pressed — the resolver or the banner — from any focus; the draft message is kept |`
   Write it before 19's binding lands.
2. **Proposed change P11 (renumbered by the conductor: P10 is Amendment 3's) — not granted in 19.** Add to §2.6b:
   `| \`↑\` / \`↓\` | In the file list, move between files (§2.2); in the body, move the text cursor |`
   `| \`PgUp\` / \`PgDn\` | Scroll the summary a page (§2.1's navigation globals) |`
   `| \`Shift+Tab\` | Move focus backwards: body → subject → files → body (§2.2's "previous pane", inside the modal) |`
   - Why: §2.2 gives `↑↓` and `Shift+Tab` these meanings everywhere, and §2.1 :44 already calls `PgUp`/`PgDn` live in every pane.
   - Also: §2.2's table (:155-171) has no `PgUp`/`PgDn` row, although §2.1 :44 names one. Add `| \`PgUp\` / \`PgDn\` | Page up / down |` there.
   - Trade-off: none found. Without it, a long summary scrolls only by `j`/`k` and the wheel.
   - If approved after 19 is built, it's a small follow-up in graft (keys, hints, `BOUND_ACTIONS`).
3. **`docs/design-notes/m6-conflicts.md:584-590`**, replace the finish block with:
   ```
   pub struct Choices { pub takes: Vec<Option<graft_git::conflict::Take>>, pub whole: Option<graft_ops::operation::conflict::Side>, pub edited: bool }
   pub struct AppState { …, pub choices: BTreeMap<PathBuf, Choices>, pub finish: Option<FinishState>, pub finish_draft: Option<(String, String)> }   // (subject, body); apart from commit_draft (ticket 19 S7)
   pub struct FinishState { pub summary: Loading<Arc<FinishSummary>>, pub job: Option<JobId>, pub cursor: usize, pub top: usize, pub expanded: Option<(usize, Loading<Arc<DiffView>>)>, pub diff_job: Option<JobId>,
       pub subject: TextField, pub body: TextArea, pub focus: FinishFocus, pub return_to: ReturnTo, pub committing: Option<JobId>, pub editing: Option<JobId>, pub notice: Option<(String, bool)> }
   pub enum FinishFocus { Files, Subject, Body }   pub enum ReturnTo { Banner, Resolver }
   pub fn open_finish(state: &mut AppState, commands: &mut Vec<Command>, from: ReturnTo, next_id: &mut impl FnMut() -> JobId) -> bool;
   pub fn finish_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress, next_id: &mut impl FnMut() -> JobId) -> bool;    // modal; before resolver_key
   pub enum GraphOverlay { Finish, … }   pub enum Command { …, ReadConflictSet(JobId), ReadFinishSummary(JobId), ReadFinishDiff(JobId, PathBuf) }   // own diff route (ticket 19 S4)
   pub enum Product { …, ConflictSet(Result<Arc<ConflictSet>, String>), FinishSummary(Result<Arc<FinishSummary>, String>), FinishDiff(Result<Arc<DiffView>, String>) }
   ```
4. **Design note :450 and :473.** Replace `fix-finish-summary-screen` Part B with `fix-finish-from-the-resolver`.
5. **Ticket 17, Scope 1, last bullet.** Replace it with: "`AppState::choices` is ticket 19's; `O`/`T` land through `take_landed` as `TakeAt::Every`, which 19 records. Nothing here."
6. **New spec** `finish-from-the-resolver.md`, written beside this one. It numbers after 19.
