# fix-finish-summary-screen: `c` on a merge shows what will land — what you resolved, what git combined on its own, the message — and `⏎` commits it

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 09-ops-conflict-set-and-summary.md, 10-resolver-line-numbers.md
Was: draft · new 2026-09-23. **Part A is approved** (the user, field notes 2026-09-23: *"yes build it that way"*). §2.6b keymap rows (P9) are to be written before the binding lands. **Part B is gated on P8** (`c` inside the resolver; the resolver stays open after the last file). Do not build Part B until P8 is approved.
Build with: blank — brownfield (ADR-0035): **scout first** (banner `c`, commit editor fields, overlay precedence). Expected **Opus**: a new modal overlay with a text field, a job and an expandable diff.
Needs: `fix-ops-conflict-set-and-summary` (`finish_summary`, `FinishSummary`, `Resume::ContinueWith`); `fix-resolver-line-numbers` (`SideNames`, for the row wording).
Crates: graft-ui, graft.

## Read first
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Need (his words on finishing), Journeys J6, J6b, J10 and "Where each event lands him", Interfaces (the finish blocks), Hard choice 3, Where I'd differ P8/P9, New UI "Finish summary" (whole: layout, row wording, focus, every state).
- `docs/field-notes.md` 2026-09-23, last entry.
- `docs/design/02-keymap.md` §2.2, §2.6a, §2.10 (the keys reused).
- `crates/graft/MAP.md` §`update::banner` (`banner_key`, `resume`, `resumed`, `continue_refusal`), §`update::commit_editor` (`CommitEditor`, `editor_key`, `paste`, `EditCommitMessage`), §update (`AppState::text_field_open`, `modal_owns_keys`, `graph_overlay`, `GraphOverlay`, `Command::ReadFileDiff`), §`update::resolver` (`resolved_advance`); `crates/graft-ui/MAP.md` `commit_editor`, `text_input`, `detail_pane` (the diff body), `graph_screen::hints` (`HintContext`, `every_context`, `coverage_slot`).
- **Scout questions:**
  1. What `c` on the focused banner calls today and where its refusal words come from.
  2. How the commit editor's `Esc` keeps a draft.
  3. Whether the detail pane's diff body can be drawn into an arbitrary `Rect` from `DiffView`.
  4. How `ReadFileDiff`'s `DiffKey`/`FileEntry` are built for a staged path.

## Scope
**Part A: the summary (approved).**
1. **Open.** `c` on the banner, for a merge with nothing unmerged, calls `open_finish(ReturnTo::Banner)` instead of `resume(Continue)`. Rebase, cherry-pick and revert keep today's `c` (gap row → M7: no summary for them). With files unmerged, today's refusal stands. `open_finish` sets `AppState::finish`, restores `finish_draft` if any, and issues `Command::ReadFinishSummary`.
2. **Draw** (`graft-ui/src/finish_summary.rs`), per the note's mockups:
   - Title `Finish · merging <from> → <into> · nothing is written until ⏎`.
   - `RESOLVED BY YOU · n`: each row's wording comes first from `AppState::choices`, else from `ResultIs` (the note's list).
   - `COMBINED BY GIT ON ITS OWN · n · space shows the change`: `path  <change>  +a −r`.
   - `MESSAGE · ⇥ edit`.
   - One column below `TWO_COLUMN_MIN` (160). At 160 and wider, the file lists on the left (60 %) and the message on the right.
   - The cursor opens on the first `by_git` row (else the first `by_you` row).
3. **`Choices`.** `AppState::choices` is written on every take answer (per block), every `ResolveWholeFile` (`whole`), and every `$EDITOR`/mergetool return (`edited`). It is cleared when the operation ends (`undo::operation_ended`) or aborts.
4. **Keys** (`finish_key`, modal, checked before `resolver_key`; `modal_owns_keys` and, for subject/body focus, `text_field_open` include it):

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
   - Message unchanged from `FinishSummary::message` → `Resume::Continue`. Changed → `Resume::ContinueWith { message }`.
   - Empty subject → `the message is empty · ⇥ to write one`, no write.
   - While `committing`, the bar reads `committing…` and keys other than `Ctrl+C` are swallowed.
   - `Done` → the summary closes, `finish_draft` is cleared, and the graph is focused with the cursor on `HEAD`'s row and `Status::Done("merged <from> into <into>")`.
   - A failure (a hook) → the summary stays open, with the reason as `notice` (problem) above the bar and the draft kept.
6. **States:**
   - loading: `⏎` → `still reading · wait`;
   - `StillUnmerged` / other read failure: `summary unavailable: <why> · ⏎ commits with git's message`, and `⏎` → `Resume::Continue`;
   - `recorded: false` → one `CHANGED BY THIS MERGE` section;
   - frame < `FINISH_MIN` (60×14) → one line `finish needs 60x14 · esc back`, and only `Esc` acts.
7. **Hints.** `HintContext::Finish { focus, expandable }`: `⏎ commit  space diff  j/k move  ⇥ message  esc back` in files focus; `⏎ commit  ⇥ body  ^E editor  esc back` in subject; `^⏎ commit  ⇥ files  ^E editor  esc back` in body. Joins `every_context`, `coverage_slot`, `BOUND_ACTIONS`.

**Part B: gated on P8.**

8. `c` in `resolver_key`: every file done → `open_finish(ReturnTo::Resolver)` for a merge, `resume(Continue)` otherwise. With files open → `<n> file(s) still conflicted · ⏎ marks done`. The resolver no longer closes after the last file: it stays with the notice `every file done ✓ · c finishes the merge`. `Esc` from the summary returns to it. The Resolver hint table gains `c finish`.

Out: rebase/cherry-pick/revert summaries; P4's `FinishMerge` (that ticket routes its `c` through `open_finish`); horizontal scroll in the diff.

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
- [ ] `…::esc_goes_back_and_keeps_the_draft`: `⇥`, type, `Esc`, `c` → the typed subject is back.
- [ ] `…::space_expands_a_git_combined_file`: one `ReadFileDiff` for that path, `DiffSpec::Staged`.
- [ ] `…::a_hook_refusal_keeps_the_summary_open`.
- [ ] `…::a_rebase_c_still_continues_directly`.
- [ ] `…::the_summary_words_what_you_chose`: `Ctrl+T ⏎` on `notes.txt`, `o` on `gone.txt` → rows `theirs · feature in 2 of 2 conflicts`, `kept · yours · main`. After a relaunch (no `choices`), the rows read `ResultIs` wording.
- [ ] graft-ui `finish_summary::tests::{one_column_below_160, two_columns_from_160, too_small_says_so}` and `hints::tests::every_context_reaches_every_shape` green.
- [ ] Part B (only if P8): `update::resolver::tests::{c_in_the_resolver_opens_the_summary, the_resolver_stays_after_the_last_file, c_with_files_open_is_refused_in_words}`.
- [ ] Captures `docs/shots/m6/fix-finish-summary-{120x40,160x48,200x50}.txt` and `…-expanded-120x40.txt`. The fixture is a two-file merge where `feature` also changes one line of `README.md` cleanly. **What right looks like:**
  - The title reads `Finish · merging feature → main · nothing is written until ⏎`.
  - `RESOLVED BY YOU · 2` lists `gone.txt` and `notes.txt` with the words above.
  - `COMBINED BY GIT ON ITS OWN · 1` lists `▸ README.md  modified  +1 −1`.
  - The message's first line is the first line of the fixture's `MERGE_MSG`. Read it with `head -1 .git/MERGE_MSG` in the fixture and quote it in the TESTING line; do not assume the wording.
  - The bar starts `⏎ commit`.
  - At 160 and 200 the message is to the right of the lists.
  - Expanded: the diff rows sit under `README.md`, indented, and the rows after them move down.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium–large. If it will not fit one context, split at the seam: the graft-ui widget and hints first, the app second. Say so, and do not compress.
