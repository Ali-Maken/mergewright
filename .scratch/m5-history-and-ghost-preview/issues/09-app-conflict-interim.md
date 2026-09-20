# app-conflict-interim: the honest interim for a conflict without a resolver — banner with focus, `CONFLICTS (n)`, mergetool/editor hand-off, Continue/Skip/Abort, relaunch into a live conflict, the undo rules around it, and `u` after a push

Status: ready
Blocked by: 07, 08
Build with: (blank — brownfield in `update/keys.rs`, `update/undo.rs`, `update/detail/keys.rs`, `update/journal.rs`, `jobs/`, `event_loop/outside.rs`; recommendation: **Opus** — a loop-owned terminal hand-over plus a journal-semantics change (live vs interrupted) whose wrong version silently closes a live rebase's entry. Testable, so not Fable.)
**Conductor set after the scouts: Opus.**
Crates: graft
Needs:
- `app-preview-and-keys` — `Operated::Conflicted { kind, paths, entry }`, `open_preview` (refuses while `in_progress`), `HISTORY_CANNOT_STOP`.
- `ops-history-operations` — `apply::{resume, Resume}`, `Recovered::live`, `Refusal::{MarkersRemain, NotInProgress, Unmerged}`, `Stage(Target::Paths)` on an unmerged path as "mark resolved".
- `git-history-reads` — `state::{in_progress_detail, InProgressDetail, mergetool}`, `history_reads::first_marker_line`.
- `ui-ghost-banner-conflict` — `BannerView`, `GraphView.banner_op`, `WorkingTreeSection::Unmerged`, `FileStatusView::Unmerged { how }`, `ConfirmView::with_choices`.
- `hint-bar-redesign` — `HintContext::{Banner { skippable }, ChangesPane { unmerged, .. }}`, `Status::Sticky { reason, keys }`.

**Spec change required before the build** (CLAUDE.md: a key the keymap does not name is not free): `docs/design/02-keymap.md` names `c s a` on the focused banner (§2.1, §2.6) but not `M` mergetool / `e` edit there, nor `M` on a `CONFLICTS` row of the changes pane. The conductor adds an "operation banner (focused), M5 interim" table to §2.6 — `M` launch the configured mergetool on the first conflicted file, `e` open `$EDITOR` at its first `<<<<<<<`, `Tab`/`Esc` leave the banner — and one line under §2.4: on a `CONFLICTS` row `M` mergetool, `Space` marks resolved (`e` is already §2.4's). The builder binds nothing the doc does not name at build time.

## Scout first (Sonnet `Explore`, ≤ 1 page) — `scout-app.md` §1–§4 first; ask only what it lacks

- `crates/graft/src/update/keys.rs` `pane_and_overlay_key` and `update/detail/keys.rs`: how `Tab`/`BackTab` move focus between graph and detail pane today; where `e` on a file row builds `OpenExternalEditor { path, line }`.
- `crates/graft/src/jobs/undo.rs` `JournalRead` + wherever `recover_interrupted` is called (launch? every read?); `update/undo.rs::interrupted_notice`; `view/journal.rs` the `interrupted` row wording.
- `crates/graft/src/event_loop/outside.rs` `Handover::edit_file` — the exact hand-over shape to copy for `git mergetool`; `event_loop/running.rs` where `Command::OpenExternalEditor` is carried out and `Product::HandedOver` produced.
- `crates/graft/src/jobs/kinds.rs`/`runner.rs`: how `ReadWorkingTree` is dispatched and what `Product::WorkingTree` carries (to add `in_progress` and `mergetool` beside the status).
- `graft_git::status::WorkingTreeStatus`: how unmerged entries are represented (the `XY` pair for `both modified` / `deleted by them` / `added by us`).
- Report: which `Status` variant `update::remote::fetch_key` uses for a refusal in words (reuse it for `u` during an operation).

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — Hard choice 2; Proposed changes 2, 3; New UI "Operation banner + conflict interim" (the mockups are the acceptance) and "`u` after a push"; Hint bar rows "Banner focused" and "Changes pane".
- PROMPT.md §8 the no-resolver paragraph (line 599), §9 (relaunch after a crash); `docs/design/01-view-mockups.md` §1.13; `docs/design/02-keymap.md` §2.1 (the banner is modal when focused), §2.6 (after the conductor's edit); `docs/design/06-undo-journal-design.md` §6.4 (conflicted operation = one entry).
- `docs/adr/0037-…md` §3 (`u` after a push says so).
- `.scratch/m5-history-and-ghost-preview/scout-app.md` §1, §2, §3; `scout-ops.md` §1 "Journal shape and states".
- `crates/graft/MAP.md` §update (`AppState`, `Command`, `Product`, `Status`), §update::undo, §update::remote (`pushed_since`, `push_box`, `RemoteState`), §update::journal, §update::confirm, §view (`journal_view`), §event_loop::outside, §jobs; `crates/graft-ops/MAP.md` §apply (`resume`, `recover_interrupted`), §journal (after G5); `crates/graft-git/MAP.md` §state, §history_reads (after G4); `crates/graft-ui/MAP.md` §graph_screen (`BannerView`), §detail (after G6).
- `KNOWN_GAPS.md` row 236 (the no-resolver row this group's NOTES re-words with expiry M6), row 218.

## Scope

In, in order (a commit per piece; stop seam after piece 4):

1. **Facts** — `ReadWorkingTree`'s job also reads `in_progress_detail` and `mergetool`; `Product::WorkingTree` carries `WorkingTreeRead { status, in_progress, mergetool }`; `AppState::in_progress: Option<InProgressDetail>`, `AppState::mergetool: Option<String>`. `unmerged_paths(state) -> Vec<PathBuf>` from the status. Read at launch, after every write, on `Ctrl+R` (already the job's cadence).
2. **Banner** (`update/banner.rs`, `view/rows.rs`): `GraphView.banner_op = banner_view(state)` whenever `in_progress` is `Some`: kind word (`MERGING` / `REBASING` / `CHERRY-PICKING` / `REVERTING`), `step` from the detail, `conflict` = first unmerged path, `more_conflicts`, `focused = state.banner_focus`, `skippable = kind != Merge`. **Focus**: `banner_focus` is set on the event that lands `Operated::Conflicted` and on launch when piece 5 finds a live entry; cleared by `Tab`/`BackTab`/`Esc` from the banner and whenever `in_progress` becomes `None`. `Tab` from the graph pane while a banner shows and the detail pane is hidden → banner; with the pane shown the cycle is graph → detail → banner → graph. **Chain position**: `banner_key` goes right after `preview_key` (before `context_menu_key`); modal while focused — claims every key but `Ctrl+C`: `c` continue, `s` skip (rebase/cherry-pick/revert; on a merge `s` reads `nothing to skip in a merge`), `a` abort, `M` mergetool, `e` edit, `Tab`/`BackTab`/`Esc` leave focus; everything else swallowed. `hint_context = Banner { skippable }` while focused. The banner's mouse buttons (≥ 160 columns, G6) call the same functions.
3. **The three verbs** — `c`: `continue_refusal(state)` first (`N files still conflicted · M mergetool  e edit  space marks resolved`, no job); else `operate::launch`-shaped `Command::Resume(id, Continue)` with `Stoppable::No(HISTORY_CANNOT_STOP)`, `Busy("continue <kind>")`. `s`: confirm box `Skip this commit?` / `<sha7> "<subject>" is left out of the result` / command `git <kind> --skip` / `[ Cancel ] [ Skip ]` → `Resume(id, Skip)`. `a`: confirm box `Abort the <kind>?` / `everything goes back to before it started · the journal keeps the entry` / `git <kind> --abort` / `[ Cancel ] [ Abort ]` → `Resume(id, Abort)` — **always offered**, whatever else is broken. `jobs::operate::run_resume` calls `graft_ops::apply::resume`; answer `Product::Resumed(Box<Operated>)`: `Done` → `Status::Done("<kind> finished")` / `("<kind> aborted")`, `banner_focus = false`, refresh `Everything`; `Conflicted` again (rebase stops on the next commit) → banner stays, `Sticky { reason: "stopped again: conflicts in <n> files", keys: Some("M mergetool  e edit  a abort") }`; `Failed` → `Sticky` with `graft_ops`' words and `a abort` as the key.
4. **Hand-offs** — `M`: `state.mergetool` is `None` → refusal `no merge.tool configured · e edits <path> instead`, no hand-over; else `Command::OpenMergetool { path }` carried out by the loop like `OpenExternalEditor`: `Handover::mergetool` runs `git mergetool --no-prompt -- <path>` in `workdir` with the terminal handed over (`run_outside_terminal`), answers `Product::HandedOver`; a non-zero exit → `Sticky { reason: "mergetool exited <code> for <path>", keys: Some("e edit  a abort") }`; then `ReadWorkingTree`. `e`: `OpenExternalEditor { path, line: first_marker_line(path) }` (line `None` when no marker: the file is already clean). **Changes pane** on a `CONFLICTS` row: `M`/`e` the same for that row's path; `Space` → `Operation::Stage(Target::Paths[path])` → G5 refuses `MarkersRemain` → `Sticky { reason: "<path> still has conflict markers at line <n>", keys: Some("e edit") }`; success → the row moves to STAGED and the count drops. `hint_context = ChangesPane { unmerged: true, .. }`. `Ctrl+A` keeps skipping unmerged paths (m3/04 M2).
5. **Journal semantics** (scout fact 3 — on disk nothing changes: a live conflict is a `Pending` line with no `Outcome` line, the same bytes an interrupted entry has; the difference is decided at read time against git's own state, so entries written by older versions need no migration and are closed by `recover_interrupted` exactly as before unless git says their kind is live now) — `JournalRead.live: Vec<EntryId>` from `Recovered::live`; `interrupted_notice` excludes them; `journal_view` words a live entry `in progress — c continue  a abort` (never `interrupted`, never `← u`); `AppState::live_entries` set with each read. **Launch**: `live` non-empty and `in_progress` `Some` → `banner_focus = true`, `Sticky { reason: "<kind> still in progress from a previous run", keys: Some("c continue  a abort") }` (PROMPT §9). The M3 harness scenario that relaunches into an interrupted entry must still close it (its kind is not live).
6. **Undo rules** — `undo_key` while `in_progress` is `Some` → refusal in words `finish or abort the <kind> first · ⇥ banner` (the variant the scout names), no job: a `Pending` entry is not in force, so `u` would otherwise undo the entry *before* the live one mid-rebase. `CommitEditor::refusal` during an operation names the keys, not only the shell command: `<kind> in progress · ⇥ banner: c continue  a abort`. `open_preview`'s refusal (previous group) stays.
7. **`u` after a push** (ADR-0037 §3; `update/undo.rs` + `update/remote.rs`): when the entry `next_undo` names was pushed since (the predicate `pushed_since` already uses) and `RemoteState.undo_after_push_asked != Some(entry.id)`, `u` opens `ConfirmView::with_choices([Cancel, Undo locally, Re-push old tip])`: title `Undo after a push`, lines `commit "<subject>" is on <upstream> (pushed <age> ago)`, `undo works on your local branch only`, `re-push puts <upstream> back to <old7> and leaves your branch as it is`, command empty; `ConfirmThen::UndoAfterPush { entry, push: PushParams }`. `Undo locally` → the normal `Command::Undo` (`asked` recorded, so the second `u` on the same entry never asks again). `Re-push old tip` → the existing lease box (`push_box`) for `PushParams { source_commit: Some(<push entry's refs_before[tracking]>), lease: Lease { expected: <push entry's refs_after[tracking]> }, .. }`, consequence `rewrites <upstream> for everyone who has <new7>`; `Enter` there launches it as today. `Cancel` → nothing, `asked` **not** recorded (Esc must be free). No push newer than the entry → `u` behaves as before (KNOWN_GAPS 486's memory-only `last_rejected_push` is untouched — M7).
8. Tests (headless harness over the real pool, `tests/it/conflict_interim.rs`; each conflict built in the test with `graft_fixtures::git::run` — two branches editing one line):
   - `conflict_focuses_the_banner_and_lists_the_files` — `m` → preview → confirm → `Operated::Conflicted`; `banner_focus`, `GraphView.banner_op` names the path, changes pane `CONFLICTS (1)` first, `MERGE_HEAD` present, the journal's last entry `Pending`
   - `banner_is_modal_when_focused` — `u`, `j`, `S`, `M`-as-map do nothing; `Tab` leaves; `Ctrl+C` passes
   - `continue_refuses_while_unmerged_then_finishes` — `c` → the refusal words, no job; write the file (test), `Space` on the row → `MarkersRemain` sticky while markers remain, staged once clean; `c` → `Committed`, `in_progress None`, `banner_focus false`; `u` → refs and tree equal the pre-merge state (`for-each-ref`, `write-tree`, bytes)
   - `abort_always_offered_and_restores` — `a` → box `[ Cancel ] [ Abort ]` → `Aborted("aborted")`, tree byte-identical, `MERGE_HEAD` gone
   - `skip_on_a_merge_says_nothing_to_skip`; `rebase_stops_twice_same_entry_id`
   - `mergetool_absent_refuses_in_words`; `mergetool_present_hands_over` (headless: `Handover` is `None` → `HandedOver(Err)` worded, not a panic; the real hand-over is the user's by-eye check, listed in TESTING.md)
   - `e_opens_the_editor_at_the_first_marker` — `OpenExternalEditor { line: Some(n) }` with `n` from `first_marker_line`
   - `relaunch_into_a_live_conflict` — start a rebase that conflicts, drop the harness, open a new one on the same worktree: `banner_focus`, sticky names the kind, the entry still `Pending` (not closed), `live == [id]`; `a` aborts it
   - `interrupted_entry_without_live_state_is_still_closed` — the M3 recovery path unchanged
   - `u_refused_during_an_operation`; `journal_row_for_live_entry_says_in_progress`
   - `u_after_push_asks_once_then_undoes_locally` (`LocalRemote`); `u_after_push_repush_old_tip_moves_the_bare_back` — bare's `main` == the push entry's `refs_before[tracking]`, local `main` unchanged; `u_after_push_cancel_asks_again`
9. MAP.md (`update::banner`, `update::undo`, `update::remote`, `jobs::operate`, `event_loop::outside`, `view::journal`); NOTES: the KNOWN_GAPS row 236 re-wording for the conductor — *"M5 detects a conflict, lists the files, hands off to `git mergetool`/`$EDITOR`, keeps Continue/Skip/Abort; **no in-app resolution**; expiry M6"* — marked unverified-by-design, not broken; **captures** ending the visual work: `docs/shots/m5-ui-banner-conflict-{80x24,120x40,160x48}.txt` (banner focused, `CONFLICTS (2)` open) and `m5-ui-undo-after-push-80x24.txt`, ≤ 8 KB each, via the pty pipeline; the user judges them.

Out: any three-pane resolver, hunk-level `o`/`t`/`b`, `n`/`N` (M6); `q` confirming during an operation (M8, KNOWN_GAPS); `last_rejected_push` persistence (M7); bisect banners.

## Signatures

```rust
// graft/src/update/mod.rs
pub struct AppState { …, pub in_progress: Option<graft_git::state::InProgressDetail>, pub mergetool: Option<String>, pub banner_focus: bool, pub live_entries: Vec<EntryId> }
pub enum Command { …, Resume(JobId, graft_ops::apply::Resume), OpenMergetool { path: PathBuf } }
pub enum Product { …, WorkingTree(Result<Arc<WorkingTreeRead>, String>), Resumed(Box<crate::jobs::operate::Operated>) }
pub struct WorkingTreeRead { pub status: graft_git::status::WorkingTreeStatus, pub in_progress: Option<InProgressDetail>, pub mergetool: Option<String> }
pub fn unmerged_paths(state: &AppState) -> Vec<PathBuf>;
// graft/src/update/banner.rs
pub fn banner_view(state: &AppState) -> Option<graft_ui::graph_screen::BannerView>;
pub fn banner_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress, next_id: &mut impl FnMut() -> JobId) -> bool;   // true whenever focused
pub fn banner_click(state: &mut AppState, commands: &mut Vec<Command>, column: u16, row: u16, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn continue_refusal(state: &AppState) -> Option<String>;
pub fn first_conflicted(state: &AppState) -> Option<PathBuf>;
pub fn resume(state: &mut AppState, commands: &mut Vec<Command>, what: graft_ops::apply::Resume, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn resumed(state: &mut AppState, id: JobId, outcome: crate::jobs::operate::Operated, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
pub fn conflicted(state: &mut AppState, kind: InProgress, paths: Vec<PathBuf>) -> Vec<Command>;   // replaces the previous group's sticky: focus + words
pub fn open_mergetool(state: &mut AppState, commands: &mut Vec<Command>, path: PathBuf) -> bool;
pub fn edit_conflict(state: &mut AppState, commands: &mut Vec<Command>, path: PathBuf) -> bool;   // OpenExternalEditor at first_marker_line
// graft/src/jobs/operate.rs
pub fn run_resume(repos: &RepoStore, what: graft_ops::apply::Resume, cancel: &Cancel, hooks: Hooks) -> Operated;
// graft/src/jobs/undo.rs
pub struct JournalRead { …, pub live: Vec<EntryId> }
// graft/src/event_loop/outside.rs
impl Handover { pub fn mergetool(&mut self, path: &Path) -> Result<std::process::ExitStatus, OutsideError>; }   // `git mergetool --no-prompt -- <path>` in workdir
// graft/src/update/confirm.rs
pub enum ConfirmThen { …, Resume(graft_ops::apply::Resume), UndoAfterPush { entry: EntryId, push: graft_ops::operation::PushParams } }
// graft/src/update/remote.rs
pub struct RemoteState { …, pub undo_after_push_asked: Option<EntryId> }
pub fn undo_after_push_box(entry: &JournalEntry, push: &JournalEntry, now_ms: i64) -> (graft_ui::confirm::ConfirmView, PushParams);   // pure
```

## Acceptance (every tick names its test at tick time)

- [ ] Every test in piece 8 green, named at tick time
- [ ] A conflict never writes an `Outcome` line and never closes the entry across a relaunch: `relaunch_into_a_live_conflict`; the old recovery still works: `interrupted_entry_without_live_state_is_still_closed`
- [ ] `a` is reachable from the focused banner in every conflicted state the tests build, including after a failed `c` and after a failed mergetool: `abort_always_offered_and_restores`
- [ ] `banner_key` sits right after `preview_key` in `key_event`; the keys it binds are exactly those in `02-keymap.md` §2.6's interim table (the conductor's edit is in the tree before this group starts — `git log -1 -- docs/design/02-keymap.md` in NOTES)
- [ ] Captures listed in piece 9 exist, ≤ 8 KB each; each shows the header banner text from the note's mockup and `CONFLICTS (2)` first in the pane; the user judges them
- [ ] MAP.md updated; NOTES: scout used or why not; the row-236 wording; what the real `git mergetool` hand-over could not be verified headless (listed for TESTING.md's per-terminal rows); colour unverified

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft` · `cargo test --doc -p graft`; the M3 walk's recovery scenario and the M4 walk unchanged.

## Size

One context. Stop seam after piece 4 (facts, banner, verbs, hand-offs) with 5–7 in a handoff note. Files: `update/banner.rs` (new), `update/keys.rs`, `update/undo.rs`, `update/remote.rs`, `update/confirm.rs`, `update/journal.rs`, `update/detail/keys.rs`, `update/mod.rs`, `view/rows.rs`, `view/journal.rs`, `jobs/operate.rs`, `jobs/undo.rs`, `jobs/kinds.rs`, `event_loop/outside.rs`, `event_loop/running.rs`. Same crate as `app-preview-and-keys` and `app-drag-drop-and-stash-branch`: sequential, never concurrent.

## Conductor: user conditions (ADR-0038 §3, §4) — binding

- **Keys:** `M` opens the mergetool, `e` edits. `e` already means "open in `$EDITOR`" in the graph pane and **keeps that one meaning everywhere**, banner and CONFLICTS row included. Write both into `docs/design/02-keymap.md` **in this same change** — the keymap is the single source of truth (ADR-0037 §1), so a key bound here and not written there is an incomplete ticket.
- **"Re-push old tip"** puts the remote branch back and keeps the user's local commits. All three conditions hold: `--force-with-lease` only, never a bare force; its own explicit confirm; and the box **says plainly that this changes the remote other people may have already pulled**, labelled so it cannot be mistaken for a local undo.
- Acceptance names the test for each condition, and the capture shows the box's wording for the user's eye.
