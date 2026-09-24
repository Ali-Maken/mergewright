# fix-finish-summary-screen: `c` on a merge shows what will land — what you resolved, what git combined on its own, the message — and `⏎` commits it

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 09-ops-conflict-set-and-summary.md, 10-resolver-line-numbers.md, 22-ops-finish-message-and-step-bound.md, 13-resolver-file-list.md (Part B)
Was: draft · new 2026-09-23. **Part A is approved** (the user, field notes 2026-09-23: *"yes build it that way"*). §2.6b keymap rows (P9) are to be written before the binding lands. **Part B is gated on P8** (`c` inside the resolver; the resolver stays open after the last file). Do not build Part B until P8 is approved.
**Revised 2026-09-23 (review `docs/reviews/m6-amend-09.md` Minor 1, the conductor's direction).** The message the user sees is the message that lands, character for character.
- git's comment lines come out when `MERGE_MSG` is **read**: `finish_summary`, as today.
- The commit keeps whatever the user typed: `ContinueWith` now sends `--cleanup=whitespace`. That change is in graft-ops, in `ops-finish-message-and-step-bound`. This ticket proves it end to end.
- A `#123 fixes login` line the user types survives.
**Revised 2026-09-24 (spec-only pass on `scout-13.md`, rulings R4/R5 in `13-resolver-file-list.md`).** Part B **owns** "the resolver stays open after the last file" (`02-keymap.md:269`). Until it lands, `fix-resolver-file-list` closes to the banner with the words on the status line. Part B replaces that close, moves the words into the resolver's notice row, and rewrites 13's two landing tests. Part B also needs 13 (`resolved_advance` in list order).
Build with: blank — brownfield (ADR-0035): **scout first** (banner `c`, commit editor fields, overlay precedence). Expected **Opus**: a new modal overlay with a text field, a job and an expandable diff.
Needs: `fix-ops-conflict-set-and-summary` (`finish_summary`, `FinishSummary`, `Resume::ContinueWith`); `ops-finish-message-and-step-bound` (`--cleanup=whitespace`); `fix-resolver-line-numbers` (`SideNames`, for the row wording); Part B only: `fix-resolver-file-list` (`resolved_advance`'s none-left branch, its landing tests).
Crates: graft-ui, graft.

## Read first
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Need (his words on finishing), Journeys J6, J6b, J10 and "Where each event lands him", Interfaces (the finish blocks), Hard choice 3, Where I'd differ P8/P9, New UI "Finish summary" (whole: layout, row wording, focus, every state).
- `docs/field-notes.md` 2026-09-23, last entry.
- `docs/design/02-keymap.md` §2.2, §2.6a, §2.10 (the keys reused).
- `crates/graft/MAP.md` §`update::banner` (`banner_key`, `resume`, `resumed`, `continue_refusal`), §`update::commit_editor` (`CommitEditor`, `editor_key`, `paste`, `EditCommitMessage`), §update (`AppState::text_field_open`, `modal_owns_keys`, `graph_overlay`, `GraphOverlay`, `Command::ReadFileDiff`), §`update::resolver` (`resolved_advance`); `crates/graft-ui/MAP.md` `commit_editor`, `text_input`, `detail_pane` (the diff body), `graph_screen::hints` (`HintContext`, `every_context`, `coverage_slot`).
- Part B: `.scratch/m6-conflicts/issues/13-resolver-file-list.md` rulings R4, R5 and Scope 7.
- **Scout questions:**
  1. What `c` on the focused banner calls today, and where its refusal words come from.
  2. How the commit editor's `Esc` keeps a draft.
  3. Whether the detail pane's diff body can be drawn into an arbitrary `Rect` from `DiffView`.
  4. How `ReadFileDiff`'s `DiffKey`/`FileEntry` are built for a staged path.
  5. Whether the `EditCommitMessage` path adds comment lines (a help template) to the file it hands `$EDITOR`.

## Scope
**Part A: the summary (approved).**
1. **Open.** `c` on the banner, for a merge with nothing unmerged, calls `open_finish(ReturnTo::Banner)` instead of `resume(Continue)`.
   - Rebase, cherry-pick and revert keep today's `c` (gap row → M7: no summary for them).
   - With files unmerged, today's refusal stands.
   - `open_finish` sets `AppState::finish`, restores `finish_draft` if any, and issues `Command::ReadFinishSummary`.
2. **Draw** (`graft-ui/src/finish_summary.rs`), per the note's mockups:
   - Title `Finish · merging <from> → <into> · nothing is written until ⏎`.
   - `RESOLVED BY YOU · n`: each row's wording comes first from `AppState::choices`, else from `ResultIs` (the note's list).
   - `COMBINED BY GIT ON ITS OWN · n · space shows the change`: `path  <change>  +a −r`.
   - `MESSAGE · ⇥ edit`.
   - One column below `TWO_COLUMN_MIN` (160). At 160 and wider, the file lists on the left (60 %) and the message on the right.
   - The cursor opens on the first `by_git` row (else the first `by_you` row).
3. **`Choices`.** `AppState::choices` is written on every take answer (per block), every `ResolveWholeFile` (`whole`), and every `$EDITOR`/mergetool return (`edited`). It is cleared when the operation ends (`undo::operation_ended`) or aborts.
4. **Keys** (`finish_key`, modal, checked before `resolver_key`; `modal_owns_keys` includes it, and so does `text_field_open` while the subject or body has focus):

   | Focus | Key | Does |
   |---|---|---|
   | files | `j`/`k`, `↑`/`↓` | move the cursor |
   | files | `space` | expand/collapse that row's diff (`ReadFileDiff` for the staged path), inline, ≤12 rows then `… N more lines · space hides` |
   | any | `⇥` / `⇧⇥` | files → subject → body |
   | files, subject | `⏎` | commit |
   | body | `⏎` | newline |
   | any | `Ctrl+Enter` | commit |
   | any | `Ctrl+E` | `$EDITOR` on the whole message (`EditCommitMessage` path) |
   | any | `Esc` | close; the draft goes to `finish_draft`; focus returns to `return_to` |
   | any | `PgUp`/`PgDn`, wheel | scroll |
   | any | `Ctrl+C` | quit |

   A click on a row moves the cursor there and toggles its diff.
5. **Commit.**
   - Message unchanged from `FinishSummary::message` → `Resume::Continue`. Changed → `Resume::ContinueWith { message }`, sent as typed: the app strips nothing.
   - `Ctrl+E`: what `$EDITOR` returns is the user's message.
     - If the `EditCommitMessage` path writes comment lines of its own into the file (scout question 5), strip exactly those lines on return, matched as the whole lines it wrote.
     - Never strip a line by its first character: the user's `#123` stays.
   - Empty subject → `the message is empty · ⇥ to write one`, no write.
   - While `committing`, the bar reads `committing…` and keys other than `Ctrl+C` are swallowed.
   - `Done` → the summary closes, `finish_draft` is cleared, and the graph is focused, with the cursor on `HEAD`'s row and `Status::Done("merged <from> into <into>")`.
   - A failure (a hook) → the summary stays open, with the reason as `notice` (problem) above the bar and the draft kept.
6. **States:**
   - loading: `⏎` → `still reading · wait`;
   - `StillUnmerged` / other read failure: `summary unavailable: <why> · ⏎ commits with git's message`, and `⏎` → `Resume::Continue`;
   - `recorded: false` → one `CHANGED BY THIS MERGE` section;
   - frame < `FINISH_MIN` (60×14) → one line `finish needs 60x14 · esc back`, and only `Esc` acts.
7. **Hints.** `HintContext::Finish { focus, expandable }`:
   - files focus: `⏎ commit  space diff  j/k move  ⇥ message  esc back`;
   - subject focus: `⏎ commit  ⇥ body  ^E editor  esc back`;
   - body focus: `^⏎ commit  ⇥ files  ^E editor  esc back`.

   It joins `every_context`, `coverage_slot` and `BOUND_ACTIONS`.

**Part B: gated on P8.**

8. `c` in `resolver_key`:
   - Every file done → `open_finish(ReturnTo::Resolver)` for a merge, `resume(Continue)` otherwise.
   - Files still open → `<n> file(s) still conflicted · ⏎ marks done`.
   - The resolver no longer closes after the last file (this replaces `fix-resolver-file-list`'s none-left close in `resolved_advance`). It stays on the file just done, and `Esc` from the summary returns to it.
   - The words move from the status line to the resolver's **notice row** (`ResolverState::notice`, drawn under the file identity row): `every file done ✓ · c finishes the merge` for a merge; `every file done ✓ · c continues the rebase` (or `the cherry-pick`, `the revert`) otherwise.
   - A **stash pop** keeps 13's behaviour: it closes to the graph with `every file done ✓` on the status line. There is nothing for `c` to finish, so staying would cost him an `Esc` for nothing.
   - The Resolver hint table gains `c finish`.

Out: rebase/cherry-pick/revert summaries; P4's `FinishMerge` (that ticket routes its `c` through `open_finish`); horizontal scroll in the diff; the cleanup mode itself (graft-ops, `ops-finish-message-and-step-bound`).

## Signatures
```rust
// graft-ui/src/finish_summary.rs (new)
pub struct SummaryRow { pub path: SanitizedText, pub what: SanitizedText }
pub enum SummaryFocus { Files, Subject, Body }
pub struct FinishView<'a> { pub title: SanitizedText, pub by_you: Vec<SummaryRow>, pub by_git: Vec<SummaryRow>, pub recorded: bool, pub cursor: usize,
    pub expanded: Option<(usize, Arc<DiffView>)>, pub subject: FieldView<'a>, pub body: AreaView<'a>, pub focus: SummaryFocus, pub notice: Option<(SanitizedText, bool)>, pub busy: bool }
pub struct FinishScreen<'a> { pub view: &'a FinishView<'a>, pub theme: &'a Theme, pub ascii: bool }
pub const FINISH_MIN: (u16, u16) = (60, 14);
pub const TWO_COLUMN_MIN: u16 = 160;
pub enum HintContext { /* existing */ Finish { focus: SummaryFocus, expandable: bool } }
// graft/src/update/finish.rs (new)
pub struct FinishState { pub summary: Loading<Arc<FinishSummary>>, pub job: Option<JobId>, pub cursor: usize, pub expanded: Option<(usize, Loading<Arc<DiffView>>)>,
    pub subject: TextField, pub body: TextArea, pub focus: FinishFocus, pub return_to: ReturnTo, pub committing: Option<JobId> }
pub enum FinishFocus { Files, Subject, Body }
pub enum ReturnTo { Banner, Resolver }
pub fn open_finish(state: &mut AppState, commands: &mut Vec<Command>, from: ReturnTo, next_id: /* the banner's existing type */) -> bool;
pub fn finish_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress, next_id: /* same */) -> bool;
pub struct Choices { pub takes: Vec<Option<Take>>, pub whole: Option<Side>, pub edited: bool }
pub struct AppState { /* existing */ pub choices: BTreeMap<PathBuf, Choices>, pub finish: Option<FinishState>, pub finish_draft: Option<(String, String)> }
pub enum GraphOverlay { /* existing */ Finish }   // precedence: before Resolver
pub enum Command { /* existing */ ReadFinishSummary(JobId) }
pub enum Product { /* existing */ FinishSummary(Result<Arc<FinishSummary>, String>) }
```

## Acceptance (name the verifying test when you tick; truth from git)
- [ ] graft `update::finish::tests::c_on_a_resolved_merge_opens_the_summary_not_the_commit`: no `Resume` command, `finish` is `Some`, `ReadFinishSummary` issued.
- [ ] `…::enter_commits_with_gits_message_when_unedited` (`Resume::Continue`) and `…::an_edited_subject_commits_with_that_message` (`ContinueWith`, with `git log -1 --format=%B` read back in the walk harness).
- [ ] `…::a_hash_line_typed_in_the_body_lands`, in the walk harness against real git: `⇥ ⇥` into the body, type `#123 fixes login`, `Ctrl+Enter` → `git log -1 --format=%B` contains the line `#123 fixes login`.
- [ ] `…::a_hash_line_from_the_editor_lands`: `Ctrl+E` with a fake `$EDITOR` that appends `#123 fixes login` → the body holds that line, and after `Ctrl+Enter` `%B` contains it.
- [ ] `…::the_summary_shows_no_git_comment_lines`: the fixture's `MERGE_MSG` has git's `# Conflicts:` block → no line of the drawn message starts with `#`.
- [ ] `…::esc_goes_back_and_keeps_the_draft`: `⇥`, type, `Esc`, `c` → the typed subject is back.
- [ ] `…::space_expands_a_git_combined_file`: one `ReadFileDiff` for that path, `DiffSpec::Staged`.
- [ ] `…::a_hook_refusal_keeps_the_summary_open`.
- [ ] `…::a_rebase_c_still_continues_directly`.
- [ ] `…::the_summary_words_what_you_chose`: `Ctrl+T ⏎` on `notes.txt`, `o` on `gone.txt` → rows `theirs · feature in 2 of 2 conflicts`, `kept · yours · main`. After a relaunch (no `choices`), the rows read `ResultIs` wording.
- [ ] graft-ui `finish_summary::tests::{one_column_below_160, two_columns_from_160, too_small_says_so}` and `hints::tests::every_context_reaches_every_shape` green.
- [ ] Part B (only if P8): `update::resolver::tests::{c_in_the_resolver_opens_the_summary, the_resolver_stays_after_the_last_file, c_with_files_open_is_refused_in_words}`. `the_resolver_stays_after_the_last_file` asserts the resolver is open and its notice row (not the status line) reads `every file done ✓ · c finishes the merge`. 13's `after_the_last_file_the_banner_says_c_finishes` and `…_a_rebase_says_c_continues` are rewritten to the stay-open landing (rebase: notice row `… · c continues the rebase`); `after_the_last_stash_file_the_status_says_every_file_done` stays green unchanged.
- [ ] Captures `docs/shots/m6/fix-finish-summary-{120x40,160x48,200x50}.txt` and `…-expanded-120x40.txt`. The fixture is a two-file merge where `feature` also changes one line of `README.md` cleanly. **What right looks like:**
  - The title reads `Finish · merging feature → main · nothing is written until ⏎`.
  - `RESOLVED BY YOU · 2` lists `gone.txt` and `notes.txt` with the words above.
  - `COMBINED BY GIT ON ITS OWN · 1` lists `▸ README.md  modified  +1 −1`.
  - The message's first line is the first line of the fixture's `MERGE_MSG`. Read it with `head -1 .git/MERGE_MSG` in the fixture and quote it in the TESTING line; do not assume the wording.
  - No message line starts with `#`.
  - The bar starts `⏎ commit`.
  - At 160 and 200 the message is to the right of the lists.
  - Expanded: the diff rows sit under `README.md`, indented, and the rows after them move down.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium–large. If it won't fit one context, split at the seam: the graft-ui widget and hints first, the app second. Say so, and do not compress.
