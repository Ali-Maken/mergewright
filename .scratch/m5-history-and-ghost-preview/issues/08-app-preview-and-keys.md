# app-preview-and-keys: the Ghost Preview state machine (plan job, bounded-slice re-layout, the box, confirm runs the pinned op), `m`/`r`, the `.` menu verbs and Fix…, the journal time machine, `p` on a diverged branch

Status: ready
Blocked by: 04, 07
Build with: (blank — brownfield across `update/keys.rs`, `view/rows.rs`, `update/context_menu.rs`, `update/journal.rs`, `update/remote.rs`, `jobs/`; recommendation: **Opus** — the note's "genuinely risky" group: the slice re-layout and the modal state machine are judged by the walk and the captures, not the diff. Not Fable: every hard choice is decided here.)
**Conductor set after the scouts: Opus.**
Crates: graft (+ graft-ui `context_menu.rs` for the new labels only — after G6/G7 have merged)
Needs:
- `ops-history-operations` — `Operation::{Merge,Rebase,CherryPick,Revert,Reset,Checkout,BranchDelete,TagDelete}` + params, `Tip`, `Source`, `preview::{plan, plan_revert_to, PreviewPlan, GhostCommit, Target, RefMove, HeadMove, WorktreeEffect, ConflictForecast, UndoNote}`, `PreviewError`, `apply::ApplyError::Conflicted`, `Refusal::{Moved, DirtyTree}`, `undo::revert_to`. Read `.scratch/m5-…/as-built.md` first: G5 is Fable and may have moved a name.
- `ui-ghost-banner-conflict` — `RowPaint::Ghost`, `PreviewBox`, `PreviewBoxWidget`, `preview_rows`, `PREVIEW_MIN`, `ConfirmView::with_choices`.
- `hint-bar-redesign` — `HintContext::Preview`, `Status::Sticky { reason, keys }`.
- `refresh-race` — `Command::OpenRepository(JobId, …)` (every history op refreshes `Everything` through `refresh_key`).

## Scout first (Sonnet `Explore`, ≤ 1 page) — `scout-app.md` covers routing, overlays and jobs; ask only what it did not

- `crates/graft/src/history/`: from an absolute row (`History::row_at`) to the `Commit` value `graft_graph::layout` takes — is the full commit (parents, subject, author, time) on hand for every resident row, or only the id until hydration? What `history::seeds_for` needs. What `crates/graft/src/view/strip.rs:113-124` passes to `layout` (the pattern to copy).
- `crates/graft/src/view/rows.rs` + `view/entries.rs`: where a row's `badges`, `sha`, `when`, `paint` are chosen; how `graph_rows` shrinks when a footer slot takes rows (the preview box takes rows the same way).
- `crates/graft/src/jobs/undo.rs`: `JournalRead`'s entries (ids present?); is `graft_ops::journal::state_at`/`read_state_now` reachable from a job here.
- `crates/graft/src/update/context_menu.rs` + `crates/graft-ui/src/context_menu.rs`: `ContextAction`, `MenuItem`, how labels are built, how `apply_context_action` gets the row's badges.
- `crates/graft/src/update/remote.rs`: the diverged-pull refusal line (which `Refusal`, where the words are built after G7's `Sticky { reason, keys }`).
- `graft_ops::mistake_radar::Finding` (MAP.md): does a Foxtrot finding expose the branch's previous tip and the merge's parents as `Oid`s, or only as text? `jobs::radar::RadarContext::previous_default_tips` is the fallback.

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — Promise; Interfaces (G5 block, "graft (app), pass 2" block); Hard choices 1, 3, 5; Proposed changes 1, 2; New UI "Ghost Preview" (the mockup is the acceptance), "`.` menu additions".
- `.scratch/m5-history-and-ghost-preview/scout-app.md` §1, §2, §3, §5 (whole); `scout-ops.md` §2 (the `layout` fact and the merge-parent rule).
- `docs/design/01-view-mockups.md` §1.7, §1.8 (Fix…), §1.10; `docs/design/06-undo-journal-design.md` §6.6; `docs/design/08-architecture.md` §8.2 lines 143–150; `docs/design/02-keymap.md` §2.1 (modal rule), §2.3 (`m`, `r`, `.`), §2.11.
- PROMPT.md §1.4 line 159 (Fix…), §1.5; `docs/adr/0031-…md` "What this binds on M5".
- `crates/graft/MAP.md` §update (`AppState`, `Command`, `Product`, `Status`, `text_field_open`), §update::context_menu, §update::operate (`launch`, `operated`), §update::journal, §update::remote (`RemoteState`, `Stoppable`), §view (`graph_view`), §jobs (`operate`, `queue_as`), §history; `crates/graft-ops/MAP.md` §preview, §operation, §apply, §undo (after G5); `crates/graft-ui/MAP.md` §graph_screen, §preview_box, §context_menu (after G6).
- `KNOWN_GAPS.md` rows 229 (commands from `plan`), 218 (journal browser status line).

## Scope

In, in order (each piece a commit; stop seam after piece 5 with 6–9 in a handoff note if the context runs short):

1. **`update/preview.rs`** — `PreviewState`, `PreviewSubject`, `open_preview`, `preview_key`, `preview_produced`, `confirm_preview`, `close_preview` (signatures below).
   - `open_preview` refuses in words (no job) while `operating` is `Some` (`wait for <desc> to finish`), while a preview is open, and while `AppState::in_progress` is `Some` (`finish or abort the <kind> first`). Otherwise `plan: Pending`, `Command::PlanPreview(id, subject)`, box drawn `planning: true` over unchanged rows.
   - **Chain position** (scout §1): `preview_key` goes **between step 1 (`confirm_key`) and step 2 (`context_menu_key`)** — a confirm box may open over a preview (never the reverse); the context menu closes before a preview opens. Modal: claims every key but `Ctrl+C`; `←`/`→`/`Tab`/`BackTab` move the cursor (clamped, no wrap), `Enter` acts on the choice under it, `Esc` closes (a `Pending` plan gets `CancelJob`), everything else swallowed. Mouse: a click on a button is that choice; wheel and every other click ignored while open.
   - Nothing-to-do plan (no `ref_moves`, `head: None`, `worktree: Untouched`, no ghosts — merge already up to date) → choices `[ Close ]` only. Plan `Err` → lines `preview failed: <reason>`, `[ Close ]`. Screen under `PREVIEW_MIN` → the widget's one-liner; `Enter` is a no-op, `Esc` closes.
   - `Confirm` → `Operation` subjects: `operate::launch` with the **same `Operation` value** (by `PartialEq`, tested) and `Stoppable::No(HISTORY_CANNOT_STOP)`; `RevertTo` subjects: `Command::RevertTo(id, entry)`. The preview closes on the same event; `Status::Busy(desc)`.
2. **`update/preview/slice.rs`** (pure, no `AppState`) — `build_slice` and the rules:
   - rows = every resident row in `[top − graph_rows, top + 2·graph_rows)` (clamped to what `History` holds), in order, copied as `Commit`s;
   - **anchors**: a `Real` target or a ghost's `Real` parent that is not resident is appended after the last row with its own parents cleared (the job reads them, piece 3);
   - **ghosts** (plan order kept): each `GhostCommit` becomes a `Commit` with `oid = ghost.oid`, parents mapped `Real(o) → o`, `Ghost(i) → ghosts[i].oid`, inserted **immediately above the topmost slice row among its transitive real parents** — so a rebase chain sits right above `onto`, a ghost merge right above the higher of its two parents;
   - **clipping** (scout-ops §2: every merge parent must be in the slice): any parent of any slice commit that is not in the slice is dropped from that copy; `Slice::clipped` counts them (a lane that ends at the slice edge is the accepted cost, note Hard choice 1);
   - the slice is refused (`SliceError::MissingParent(oid)`) only when a ghost's `Real` parent is neither resident nor an anchor — the box then shows `preview cannot be drawn here: <sha7> is not loaded · esc` with `[ Close ]`; never a panic.
   Then `graft_graph::layout(&slice.commits, seeds)` once, in `preview_produced`; `rows`, `slice_top`, `ghost_index` stored. Under 16 ms for 3 screens of rows (assert in a unit test with `Instant`, generous bound 50 ms debug).
3. **`jobs/preview.rs`** — `run_preview(repos, subject, cancel) -> Result<Arc<Planned>, String>`: opens git and the journal the way `jobs::operate::run_operation` does, `read_facts` (no remotes), `preview::plan(repo, git, op, &facts, cancel)` or `preview::plan_revert_to(entries, target, &now)`; then reads every `Real` target and ghost parent that is not among the rows the subject named as resident (`PreviewSubject.resident: HashSet<Oid>` filled by `open_preview`) — capped at `PREVIEW_ANCHOR_CAP = 64`, past it `MissingParent` is the answer — as `Commit`s through the app's own commit reader. `Command::PlanPreview(JobId, Box<PreviewSubject>)` queued with `queue_as`; answer `Product::Preview(Result<Arc<Planned>, String>)` under that id, stale ids dropped. This is a **read**: `tests/it/seam_rule.rs` must allow `graft_ops::preview::*` here and still forbid a write.
4. **View** — `view/rows.rs::graph_view`: while `state.preview` is `Some` with `plan: Ready`, entries come from `preview.rows`, aligned so the cursor's commit keeps its screen row; a ghost row: `RowPaint::Ghost`, badge `(ghost)` first, `sha`/`when` = `—`, author from the ghost, `compare_marked`/`marker` never set; **badge overlay** (`badges_for_preview`, pure): each `RefMove.name` removed from its `from` commit and added to its `to` (ghost or real), `HeadMove` moves the HEAD mark, a `BranchDelete`/`TagDelete` subject removes the badge from its row; `hint_context = Preview`. `view/frame.rs`: the graph area gives up `preview_rows(view, width)` rows above the footer to `PreviewBoxWidget` (never the footer row itself); pixel tier draws the preview at the text tier (note Hard choice 1). `Pending` → rows unchanged, `planning: true`. Box wording (every line from the plan, never a hand-written command — row 229):
   - title `Preview: <plan.description>`
   - refs: one line per `RefMove`: `<short name> <from7> → <to7 | ghost>`; `HeadMove::ToBranch` → `HEAD → <branch>`; `Detached` → `HEAD detaches at <sha7|ghost>`; ghosts present → `<n> commits replayed` / `1 merge commit` prefixed
   - worktree: `working tree: untouched` · `working tree: <n> files updated` (`, more` when `more`) · `working tree: LOSES <p1>, <p2> (+n) (backup saved)` · `index and working tree restored from backup`
   - conflicts: `may conflict (forecast): <≤3 paths> (+n)` · `no conflicts expected (forecast)` · `conflicts: not checked (<reason>)`
   - undo: `undo: u restores <ref names | the tree>` · `not undoable: <why>`; time machine adds `<n> steps`
   - commands: `plan.commands` verbatim, one per line
   - choices `[ Cancel ] [ Confirm ]`, **Cancel under the cursor** (Proposed change 1); footer `←→ choose  ⏎ act  esc cancel` (the widget's).
5. **`m` / `r`** in `graph_key` (§2.3): the row's **source** = its first local branch badge, else its first remote badge, else `Source::Commit(oid)`. `m` → `Merge { source, into: Tip(HEAD's branch), source_tip: row oid, ff: Default }`; `r` → `Rebase { branch: Tip(HEAD's branch), onto: source, onto_tip: row oid }`. Refusals in words, never a silent no-op: detached HEAD `m needs a branch checked out`; the working-tree row `m needs a commit row`; the row HEAD is on `already on it`. `Tip.oid` is read from `state.refs` at key time — that is what "drawn against" means.
6. **`.` menu verbs** (`update/context_menu.rs`, labels in graft-ui `context_menu.rs`), after the existing four items, each → `open_preview`: `Checkout <b>` per local badge not current, or `Checkout <sha7> (detached)` on a row with none; `Merge <b> into <cur>` per local/remote badge (row's source rule); `Rebase <cur> onto <b|sha7>`; `Cherry-pick <sha7>` → `PickParams { commits: [oid], head }`; `Revert <sha7>` → `RevertParams { mainline: None }` (a merge row: `Revert <sha7> (-m 1)`, mainline 1); `Reset <cur> to here (mixed)` / `(hard)` → `ResetTarget::Head`; `Delete branch <b>` (never the current one) → `RefDeleteParams { expected: badge's oid }`; `Delete tag <t>`. On a **Foxtrot-marked** merge row, three more: `Fix: revert the merge (-m N)` where `N` = the parent number of the branch's previous tip (2 for a foxtrot; from the finding, else `previous_default_tips`); `Fix: reset <cur> to before the merge` → `Reset Head { mode: Hard, to: previous tip }` (PROMPT §1.4: backup ref = §6.3's snapshot); `Fix: rebase instead` → `Rebase { branch: cur, onto: Source::Commit(the other parent) }`, shown only while HEAD is on this merge. When the previous tip cannot be determined the Fix items are absent and NOTES says why. The working-tree row gets none of these.
7. **Journal time machine** — `journal_key` `Enter` on an older-than-next-undo row: close the browser, `open_preview(RevertTo(entry.id), return_to_journal: true)`; `Esc`/`Cancel` reopens the browser at the same row; `OLDER_ENTRY` deleted. `Command::RevertTo(JobId, EntryId)` → `jobs::undo::run_revert_to` → `graft_ops::undo::revert_to(ctx, target, force)`; `Product::RevertedTo(Box<RevertedTo>)`; `Status::Done("reverted to before \"<desc>\" (<n> steps)")`; `Failed { done, error }` → `Sticky { reason: "revert stopped after <done> of <n> steps: <error>", keys: Some("J journal") }`; drift → the existing drift box, then `force`. Refresh `Everything` whenever `done > 0`.
8. **`operated` for history ops** (`update/operate.rs`): `Refused(Moved { name })` → `Sticky { reason: "<short name> moved since the preview", keys: Some("<key> again") }` (`m again` / `r again` / `. again`); `Refused(DirtyTree { count })` → `Sticky { reason: "rebase needs a clean tree", keys: Some("S stash <count> changed files") }`; `ApplyError::Conflicted { kind, paths, entry }` → new `Operated::Conflicted { kind, paths, entry }` → here: `Sticky { reason: "<desc> stopped: conflicts in <n> files", keys: None }` + refresh `Everything` (the `app-conflict-interim` group replaces this line with the banner; this group must compile and behave honestly without it). `Done` → `Status::Done(desc)` + `Applied.refresh`.
9. **`p` on a diverged branch** (`update/remote.rs`): the diverged refusal becomes `Sticky { reason: "<branch> and <upstream> have diverged", keys: Some("m merge it  r rebase onto it") }` and sets `RemoteState.diverged_upstream = Some((tracking ref, tip))`; `m`/`r` use it as the source while it is `Some`, whatever row the cursor is on; cleared by `Esc` (sticky dismissed), any launched write, refresh.
10. **`HISTORY_CANNOT_STOP`** — `"<desc> runs to the end · a conflict stops it on the banner, where a aborts"`; every history op and `RevertTo` launches `Stoppable::No` (a SIGTERM mid-rebase would leave `rebase-merge/` with an entry `Aborted`, which G5 does not model). NOTES names the KNOWN_GAPS row for the conductor: "Esc during a history op = abort" → M7.
11. Tests (`tests/it/preview.rs`, headless harness over the real pool, corpus shapes + scratch commits; unit tests beside `slice.rs`):
    - `slice_places_ghosts_above_their_topmost_real_parent`, `slice_clips_parents_outside_and_counts_them`, `slice_appends_anchors_last`, `slice_is_deterministic`, `slice_refuses_a_missing_ghost_parent` (unit)
    - `preview_box_lines_come_from_plan` — `PreviewBox.commands == plan.commands` for merge, rebase, reset hard, delete branch, revert-to
    - `m_opens_preview_and_enter_runs_the_pinned_op` — the `Operation` inside `Command::Operate` equals `PreviewSubject`'s
    - `esc_closes_and_nothing_runs` — no `Command::Operate`, `for-each-ref` unchanged
    - `stale_tip_is_refused_in_words` — `git update-ref` moves `main` between `Ready` and `Enter`; sticky names `main`; refs unchanged
    - `preview_refuses_while_operating_and_while_in_progress`
    - `nothing_to_do_plan_offers_close_only`, `too_small_enter_is_a_noop` (size 38×10), `plan_failure_shows_the_reason`
    - `menu_verbs_build_expected_operations` — every verb in piece 6 → the `Operation` the table says, including the three Fix items on a foxtrot shape
    - `journal_enter_on_older_row_opens_preview_and_confirm_reverts` — three entries, `state_at(target)` equals git after, three Undo entries
    - `diverged_pull_sets_m_r_source` — `LocalRemote`, teammate commit, local commit, `p` → sticky with both keys; `r` → preview whose `onto` is the tracking ref
    - `history_ops_launch_unstoppable` — `Esc` during a running merge does not send `CancelJob`; the line says why
12. MAP.md (`update::preview`, `update::preview::slice`, `jobs::preview`, `update::context_menu`, `update::journal`, `update::remote`); `as-built.md` addendum for anything that differs from the Signatures block; **one capture** to end the visual work: `docs/shots/m5-ui-preview-rebase-80x24.txt` and `-160x48.txt` via the existing pty pipeline (`Harness::prelude` keys `r` on a branch row of a fork shape), ≤ 8 KB each — the user judges the ghost rows and the docked box; the acceptance group takes the full set.

Out: drag-and-drop, `b`/`t`, stash `b` (next group); the banner, `Resume`, mergetool, `u` after a push (`app-conflict-interim`); pixel-tier ghosts (M8); multi-commit cherry-pick; annotated tags.

## Signatures

```rust
// graft/src/update/preview.rs
pub enum PreviewSubject { Operation { op: Box<graft_ops::operation::Operation>, opened_by: &'static str /* "m", "r", "." */ }, RevertTo(graft_core::journal::EntryId) }
pub struct Planned { pub plan: graft_ops::preview::PreviewPlan, pub anchors: Vec<Commit> }   // Commit = what graft_graph::layout takes
pub struct PreviewState { pub subject: PreviewSubject, pub resident: HashSet<Oid>, pub job: JobId, pub plan: Loading<Arc<Planned>>,
    pub rows: Vec<graft_graph::Row>, pub slice_top: usize, pub ghost_index: HashMap<Oid, usize>, pub cursor: usize, pub return_to_journal: bool }
pub fn open_preview(state: &mut AppState, commands: &mut Vec<Command>, subject: PreviewSubject, return_to_journal: bool, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn preview_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress, next_id: &mut impl FnMut() -> JobId) -> bool;   // true whenever open
pub fn preview_click(state: &mut AppState, commands: &mut Vec<Command>, column: u16, row: u16, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn preview_produced(state: &mut AppState, id: JobId, result: Result<Arc<Planned>, String>);
pub fn confirm_preview(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId);
pub fn close_preview(state: &mut AppState, commands: &mut Vec<Command>);
pub fn preview_box(state: &AppState) -> Option<graft_ui::preview_box::PreviewBox>;   // the wording table, pure
pub const HISTORY_CANNOT_STOP: &str;
// graft/src/update/preview/slice.rs (pure)
pub struct Slice { pub commits: Vec<Commit>, pub ghost_at: Vec<usize>, pub clipped: usize }
pub enum SliceError { MissingParent(Oid) }
pub fn build_slice(resident: &[Commit], anchors: &[Commit], plan: &PreviewPlan) -> Result<Slice, SliceError>;
pub fn badges_for_preview(badges: &RefBadges, plan: &PreviewPlan, subject: &PreviewSubject) -> BadgeOverlay;   // pure; `BadgeOverlay::on(oid) -> Vec<RefLabel>`
pub const PREVIEW_ANCHOR_CAP: usize = 64;
// graft/src/jobs/preview.rs
pub fn run_preview(repos: &RepoStore, subject: &PreviewSubject, cancel: &Cancel) -> Result<Arc<Planned>, String>;
// graft/src/update/mod.rs
pub enum Command { …, PlanPreview(JobId, Box<PreviewSubject>), RevertTo(JobId, EntryId) }
pub enum Product { …, Preview(Result<Arc<Planned>, String>), RevertedTo(Box<crate::jobs::undo::RevertedTo>) }
pub struct AppState { …, pub preview: Option<PreviewState> }
// graft/src/jobs/undo.rs
pub enum RevertedTo { Done { steps: Vec<Applied> }, Drift { … /* the existing drift shape */ }, Failed { done: usize, total: usize, error: String } }
pub fn run_revert_to(repos: &RepoStore, target: EntryId, force: bool, cancel: &Cancel) -> RevertedTo;
// graft/src/jobs/operate.rs
pub enum Operated { …, Conflicted { kind: graft_git::state::InProgress, paths: Vec<PathBuf>, entry: EntryId } }
// graft/src/update/remote.rs
pub struct RemoteState { …, pub diverged_upstream: Option<(String, Oid)> }
// graft-ui/src/context_menu.rs
pub enum ContextAction { …, Checkout(CheckoutTarget), Merge { source: Source }, Rebase { onto: Source }, CherryPick(Oid), Revert { commit: Oid, mainline: Option<u8> },
    Reset { to: Oid, mode: ResetMode }, DeleteBranch { name: String, at: Oid }, DeleteTag { name: String, at: Oid }, FixRevert { mainline: u8 }, FixReset { to: Oid }, FixRebase { onto: Oid } }
```

## Acceptance (every tick names its test at tick time)

**Status: done — continuation 2 finished piece 6, the last unbuilt scope item.** Continuation 1 finished pieces 7, 8, 9 and 10. Pieces 1–5 came from the first builder. This one added the journal's `Enter` time machine (and **ran** the revert path for the first time), the `operated` wording for the two history refusals, `p` on a diverged branch, `HISTORY_CANNOT_STOP`'s own test, and group 06's owed acceptance box 2. Piece 6 (the `.` menu verbs and Fix…) needs new `ContextAction` variants in `crates/graft-ui/src/context_menu.rs`, which is outside this builder's fence — see **Handoff (continuation 2)**.

Ticked:

- [x] Integration tests green, no `sleep` or poll loop in any of them (they use the harness's `pump`): `tests/it/preview.rs::r_on_a_fork_draws_ghosts_for_the_commits_it_would_replay`, `::the_preview_box_and_its_ghost_rows_are_actually_drawn`, `::esc_closes_the_preview_and_the_repository_never_moved`. **Piece 11's full list is not done** — see Handoff.
- [x] The `Operation` that runs is the one previewed (the value is moved across, never rebuilt): `update::preview::tests::confirm_runs_the_very_operation_the_picture_was_drawn_from`. **Not ticked:** a moved tip never running anything — `check`'s `Moved` refusal is `graft_ops`' and the app-side test for it is not written. See Handoff.
- [x] Every string in the box comes from `PreviewPlan`: `update::preview::tests::the_box_shows_the_plans_own_command_lines` (the plan's own `commands`, compared element for element). Inspection: `grep -n '"git ' crates/graft/src/update/preview.rs crates/graft/src/update/preview/slice.rs` finds nothing — no hand-written git command anywhere in the module.
- [x] `preview_key` sits between `confirm_key` and `context_menu_key` in `key_event` (both are in `keys::boxes_key`, confirm first, and `context_menu_key` is called after it); a `u` typed while the preview is open does nothing: `update::preview::tests::an_open_preview_claims_the_keys_behind_it`.
- [x] A preview never loads more history mid-preview, and says so when the part that matters falls outside the slice — the conductor's two binding conditions: `update::preview::slice::tests::slice_refuses_a_missing_ghost_parent` (the slice half) and `update::preview::tests::a_preview_that_cannot_be_drawn_here_says_so_instead_of_drawing_it` (the box half: `preview cannot be drawn here: <sha7> is not loaded · esc`, `[ Close ]` only, nothing laid out). Clipped lanes are counted and reported: `slice_clips_parents_outside_and_counts_them`, `slice_appends_anchors_last`, `lanes_that_leave_the_view_are_counted_on_the_box`.
- [x] Every refusal is a problem line, never a `✓`: `a_preview_refuses_while_a_write_is_running_and_says_so_as_a_problem`, `a_preview_refuses_while_git_has_an_operation_in_progress`, `m_and_r_on_a_detached_head_say_what_is_missing`, `r_on_the_uncommitted_row_says_it_needs_a_commit`.
- [x] The box is really drawn, at the real size, over rows the graph really gave up: `tests/it/preview.rs::the_preview_box_and_its_ghost_rows_are_actually_drawn` (reads the drawn buffer at 100x30) and `update::preview::tests::an_open_preview_takes_its_rows_out_of_the_graph_viewport`. The click targets agree with what the widget draws: `a_click_lands_on_the_button_the_widget_drew` renders the real `PreviewBoxWidget` into a `Buffer` and finds the labels on the row `choice_at` uses.
- [x] Captures `docs/shots/m5-ui-preview-rebase-80x24.txt` (3.4 KB) and `-160x48.txt` (5.2 KB) exist, both under 8 KB, written by `tests/it/preview.rs::capture_preview_for_docs_shots` (`#[ignore]`d) from the real path. They show two ghost rows with `—` for sha and age, `feature`'s badge moved onto the newest ghost, and the docked box with `▸[ Cancel ]` first. **The user judges them; this builder does not tick "looks right".** Colour is unverified: the capture is a `TestBackend` character grid with no colour attributes (CLAUDE.md rule 5).
- [x] `m`/`r` build the right operations, pinned to the tips the picture was drawn against: `m_builds_a_merge_of_the_selected_branch_into_the_current_one`, `r_builds_a_rebase_of_the_current_branch_onto_the_selection`, `the_history_keys_are_the_plain_letters_only`. Both keys are in `docs/design/02-keymap.md` §2.3 — read before binding; no key outside what the doc gives was taken.
- [x] MAP.md updated: `update::preview`, `update::preview::slice`, `view::preview`, `jobs::preview`, `jobs::undo::{RevertedTo, run_revert_to, journal_entries}`, `update::undo::{revert_to_forced, reverted_to}`, `history::preview_seeds`, `refs::RefBadges::remotes`.

Ticked by continuation 1 (each names its verifying test at tick time):

- [x] **Piece 7 (scope item 8) — `operated` for the history operations.** `Refused(Moved)` → `Sticky { reason: "main moved since the preview", keys: Some("r again") }`; `Refused(DirtyTree { count })` → `Sticky { reason: "rebase needs a clean tree", keys: Some("S stash 3 changed files") }`: `update::operate::tests::a_refused_history_operation_says_which_key_answers_it` (both rows, exact `Status` values). Every other refusal keeps `graft_ops`' own sentence with no invented key: `update::operate::tests::a_refusal_with_no_key_to_offer_keeps_the_plain_sentence`. **Not ticked:** `Operated::Conflicted` does **not** gain `entry: EntryId` (as-built 13) and the `Done` arm was already built — neither was changed here.
- [x] **Piece 8 (scope item 7) — the journal's `Enter` time machine.** The key opens a preview of `RevertTo(entry.id)` and launches nothing: `update::journal::tests::enter_on_an_older_row_opens_a_preview_of_reverting_to_it` (asserts the subject, `return_to_journal`, the single `PlanPreview` command and `operating == None`). `Esc` puts the browser back on the same row: `update::journal::tests::the_browser_comes_back_on_the_same_row_when_the_preview_is_cancelled`. `OLDER_ENTRY` deleted (`grep -rn OLDER_ENTRY crates/` → nothing).
- [x] **The revert/time-machine write path really runs**, end to end, read back from git rather than from the status line: `tests/it/preview.rs::journal_enter_on_an_older_row_previews_and_confirm_really_reverts` — three real stages through the changes pane, `J`/`j`/`j`/`Enter`, the real `plan_revert_to`, `→`/`Enter` on `Confirm`, then `git diff --cached` empty, the three files still on disk, and six journal entries of which the last three are `OperationKind::Undo`. It also proves planning is a read (journal and index unchanged between `Enter` and `Confirm`).
- [x] **Piece 9 (scope item 9) — `p` on a diverged branch.** `remote_failed` words it `main and origin/main have diverged · m merge it  r rebase onto it` and records `("refs/remotes/origin/main", THEIRS)`; `r` then previews a rebase **onto the tracking ref**, pinned to that tip, whatever row the cursor is on: `update::remote::tests::diverged_pull_sets_m_r_source` (asserts the whole `Operation` value). `Esc` dismisses the line and the field together: `update::remote::tests::dismissing_the_diverged_line_takes_its_keys_with_it`. The M4 walk's own expectation was updated to the new line: `tests/it/m4_walk::every_scenario::pull_diverged_refused`. **Deviation:** not cleared by a refresh — as-built 12 says why, and `check`'s `Moved` is what keeps it safe.
- [x] **Piece 10 — `HISTORY_CANNOT_STOP`.** `Esc` during a running merge sends no `CancelJob` and the line says what does stop it: `update::remote::tests::history_ops_launch_unstoppable`.
- [x] **Group 06's owed acceptance box 2.** `UNDO_CANNOT_STOP` deleted (`grep -rn UNDO_CANNOT_STOP crates/` → nothing), `u`/`U` launch `Stoppable::Yes`, the busy line is `undo · esc cancels`, and `Esc` sends one `CancelJob` and latches `cancelling`: `update::remote::tests::esc_cancels_an_undo_in_flight` (both `u` and `U`). The answer side is `Undone::Cancelled` → `Status::Done` clean / `Status::Sticky` part-way (as-built 11). **Not ticked:** no test drives a *real* cancelled undo through the app — `graft-ops`' own `undo_cancel::*` suite covers the rollback, and the app side is unit-tested only.
- [x] `as-built.md` addendum — deviations 9–15.

Ticked by continuation 2 (each names its verifying test at tick time):

- [x] **Piece 6 (scope item 6) — the `.` menu verbs.** Every verb in the ticket's table, with the exact `Operation` behind it: `update::context_menu::verbs::tests::menu_verbs_build_expected_operations` (nine labels in order, then one `assert_eq!` on the whole `Operation` value per label). The two that `m`/`r` also build are the *same* values those keys build, compared against `preview::source_at`/`current_branch` directly: `::the_merge_and_rebase_verbs_are_the_ones_m_and_r_build`.
- [x] **The three Fix… items on a foxtrot shape**, all three stated against the finding's `real_tip`: `update::context_menu::verbs::tests::a_foxtrot_merge_head_is_on_offers_all_three_fixes` (the whole `Operation` value for each). `Fix: rebase instead` only while `HEAD` is on the merge: `::fix_rebase_is_absent_unless_head_is_on_the_merge`. **When the previous tip cannot be determined the Fix items are absent** (the ticket's own condition): `::no_fix_item_when_the_previous_tip_is_not_a_parent`, and a finding on another row adds nothing here: `::a_foxtrot_on_another_row_adds_nothing_here`.
- [x] **A verb that git would refuse is not offered**, rather than offered and then failing: no `Checkout`/`Delete branch`/`Merge`/`Rebase` on the current branch's own tip (`::the_current_branchs_own_row_offers_neither_checkout_nor_delete_nor_merge`, exact label list), no `Merge`/`Rebase` on a detached `HEAD` (`::a_detached_head_keeps_the_verbs_that_do_not_need_a_branch`), no verb at all on an unborn `HEAD` (`::an_unborn_head_offers_no_history_verb`), and a merge row's revert names its mainline because git refuses one without `-m` (`::a_merge_rows_revert_names_its_mainline`).
- [x] **The working-tree row gets none of these**: `update::context_menu::tests::the_working_tree_row_offers_no_history_verb` (no `HistoryVerb`, and no `ContextAction::History` item in the menu).
- [x] **Each verb → `open_preview`, and nothing runs.** `update::context_menu::tests::a_history_verb_previews_the_operation_its_label_named_and_runs_nothing` (the `PreviewSubject` is the verb's own `Operation` by value, `opened_by: "."`, one `PlanPreview`, no `Command::Operate`, `operating == None`) and `::every_history_verb_the_menu_lists_opens_a_preview` — every label the menu offers, not just one, so no verb is a label with nothing behind it. `open_preview`'s refusals reach the menu path: `::a_verb_chosen_while_a_write_runs_is_refused_in_words`.
- [x] **Through the real path, on a real repository**: `tests/it/preview.rs::the_context_menu_lists_its_history_verbs_and_draws_them` (real git, real job pool, `.` pressed through `update`, labels read off the **drawn buffer** at 100x30) and `::choosing_a_menu_verb_previews_that_operation_and_writes_nothing` (`Enter` on `Rebase feature onto main` → the real `graft_ops::preview::plan`, box drawn, and `for-each-ref` byte-identical before and after — planning is a read).
- [x] **`graft-ui` still does not name a `graft-ops` type.** `crates/graft-ui/Cargo.toml` has no `graft-ops` in `[dependencies]` or `[dev-dependencies]` (unchanged by this continuation); `ContextAction::History(usize)` carries an index, and `MenuItem::label` is `graft_term::SanitizedText`, so a ref name reaches the widget only through `term::sanitize` (`ARCHITECTURE.md:191`, `DESIGN.md` §8.2). The index means the app's own list and not the menu row, so `Explain` shifting the rows cannot change what a verb does: `graft_ui::context_menu::tests::a_history_index_names_the_apps_own_list_not_the_menu_row`, and the labels sit between the row's actions and the dropped keys: `::history_verbs_sit_between_the_rows_actions_and_the_dropped_keys` + `update::context_menu::tests::a_branch_row_lists_its_history_verbs_between_its_actions_and_the_dropped_keys`. Sanitising really happens: `update::context_menu::verbs::tests::an_invisible_character_in_a_ref_name_is_cleaned_out_of_the_label`.
- [x] **No new key bound.** `docs/design/02-keymap.md` §2.2 and §2.3 read before building: `.` is already the context-actions menu and §2.1 says rare verbs (delete branch/tag, cherry-pick, reset to here, fix a Mistake Radar marker) live behind it. This piece adds items behind `.` and binds nothing.
- [x] **Captures** `docs/shots/m5-ui-context-menu-verbs-80x24.txt` (2.6 KB) and `-160x48.txt` (3.5 KB), written by `tests/it/preview.rs::capture_context_menu_for_docs_shots` (`#[ignore]`d) from the real path — real repository, real job pool, real `.`, real frame. **The user judges them; this builder does not tick "looks right".** Colour is unverified: a `TestBackend` character grid carries no colour attributes (CLAUDE.md rule 5).
- [x] MAP.md updated: `crates/graft-ui/MAP.md` (`context_menu`) and `crates/graft/MAP.md` (`AppState::context_menu`, `update::context_menu` + `OpenMenu`/`HistoryVerb`, new `update::context_menu::verbs` section).

Not ticked (not built):

- [ ] Piece 11's remaining tests: `preview_box_lines_come_from_plan` across five operation kinds, `stale_tip_is_refused_in_words` (a real `git update-ref` between `Ready` and `Enter`), `nothing_to_do_plan_offers_close_only`, `too_small_enter_is_a_noop`, `plan_failure_shows_the_reason`. (`menu_verbs_build_expected_operations` is now done, above.)
- [ ] KNOWN_GAPS rows for the conductor (below).

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft -p graft-ui` · `cargo test --doc -p graft`; `tests/it/seam_rule.rs` still green with the preview read allowed.

## Size

One context, the largest app group. Stop seam after piece 5 (preview core + `m`/`r`), pieces 6–10 in a handoff note. Files: `update/preview*.rs` (new), `jobs/preview.rs` (new), `update/keys.rs`, `update/context_menu.rs`, `update/journal.rs`, `update/operate.rs`, `update/remote.rs`, `update/mod.rs`, `view/rows.rs`, `view/frame.rs`, `jobs/undo.rs`, `jobs/operate.rs`, `jobs/runner.rs`, graft-ui `context_menu.rs`. Runs alone in graft after G1–G3 and G6–G7 merge; the next two app groups build on it.

## Conductor: user conditions (ADR-0038 §5) — binding

- A preview **never loads more history mid-preview**. Parents outside the laid-out slice are clipped and drawn as anchors.
- **If the part of the preview that matters falls outside the slice, the preview says so** — a plain line the user can read — rather than drawing a truncated shape they could misread. Name the test that proves this.


## NOTES (builder, m5/08)

**Scout: yes.** An `Explore` scout (sonnet) covered `update/keys.rs`, `view/rows.rs`, `update/context_menu.rs`, `update/journal.rs`, `update/remote.rs` and `jobs/` before any code was written, per ADR-0035.

**Compaction: yes, once, mid-ticket.** It happened after the preview core and the slice were written and before anything compiled. Nothing was lost — the state machine, the slice and their unit tests survived and are all green — but the conductor should know the turn budget was spent twice over the same context.

**The `PartWay` seam: nothing to close, and the brief's signature was stale.** The conductor's brief described `UndoError::PartWay { done, total, error }`. The real definition is `{ done, error }` (`crates/graft-ops/src/undo/execute.rs:38`), which the conductor corrected mid-task. Verified three ways in this worktree before writing anything against it: `grep -rn 'PartWay' crates/` shows all five `crates/graft/src/jobs/undo.rs` sites already matching the two-field shape, `cargo check --workspace --all-targets` was clean at the baseline, and `cargo nextest run --workspace` was 2006/2006. **No app-side call site was left unclosed by group 07**, and no code here was written against the three-field shape, so nothing had to be undone. `jobs::undo::RevertedTo::Failed` does carry a `total`, but it reads it from `undo::plan_revert_to` **before** the run, not from `PartWay`.

**Group 06's acceptance box 2 is out of this ticket's scope and out of this worktree's base.** `esc_cancels_an_undo_in_flight` does not exist at `950ab1d`, the commit this worktree is based on; it lands in `9ff1d28`, which is already on `main` and **ahead of this base**. `UNDO_CANNOT_STOP` here still reads `"cannot be stopped part-way — finishing"`, which may be exactly what group 06 changed. Picking the test up here would be writing it against a version of `update/undo.rs` that is about to be replaced, and `update/undo.rs` is a file this ticket edits — so the conductor should expect a merge there and should file that test separately.

**Deviations from the ticket's Signatures block** (for `as-built.md`):

1. `Command::PlanPreview(JobId, Box<PreviewRequest>)`, not `Box<PreviewSubject>`. The job needs to know which commits are already on screen so it only reads the ones that are not; `PreviewRequest` is `{ subject, resident: HashSet<Oid> }`. Same for `run_preview(repos, &PreviewRequest, cancel)`.
2. `Command::RevertTo(JobId, EntryId, bool)` — the `force` flag, the way `Command::Undo` carries one. Without it the drift guard has no way to say "yes, go anyway".
3. `RevertedTo::Drift` carries `target: EntryId`. The confirm box has to aim the retry at the same entry, and inventing a field on `AppState` to remember it would have been a stand-in value for something the answer already knows.
4. `update::preview::BoxRect` instead of `ratatui::layout::Rect`. `update::tests::update_cannot_reach_the_world` forbids `update` from naming a drawing type, so the dock arithmetic is mirrored in plain numbers, the way `update::context_menu_area` already does. It is kept honest by `a_click_lands_on_the_button_the_widget_drew`, which renders the real widget.
5. `history::preview_seeds(Option<&WalkFacts>, &RefBadges)` is new. `seeds_for` takes `&[RefEntry]` + `&Branches`, and the app holds neither; without matching seeds a ghost would land in the wrong lane and the wrong colour. `RefBadges::remotes` was added for it.
6. `jobs::undo::journal_entries(repos, cancel)` is new, so the time machine's preview plans from the very entries `revert_to` will run against.
7. `keys::boxes_key` and `keys::write_key` are new private helpers. They exist only because clippy's line and cognitive-complexity limits fire on `key_event` and `graph_key` once a sixth modal and a seventh key branch are added; the ordering they encode is unchanged.
8. `Slice::clipped` now counts an **anchor's** cleared parents as well as parents clipped from resident rows. An anchor is the bottom of the picture and history plainly continues below it; not counting those was a preview quietly drawing a shorter history than the one it is about to change. `slice_appends_anchors_last` was corrected — it had asserted 1 where the honest answer is 2.

**Not done, and it is a real gap: the `(ghost)` badge.** Piece 4 asks for `(ghost)` as the first badge on a ghost row. `graft_ui::graph_screen::BadgeKind` has no ghost variant, and every existing variant carries a colour meaning (`Tag`, `Head`, `Branch`, `Remote`, `Stash`) that would be a lie on a ghost. Adding `BadgeKind::Ghost` is a `graft-ui` change and `graft-ui` is outside this builder's fence. A ghost row today is marked by the hollow node glyph, the dashed edges, and `—` in place of both sha and age — visible in the capture, but not the word the ticket asked for. **Conductor: this needs a small `graft-ui` ticket, or an explicit decision that the glyph is enough.**

**Ghost lanes are approximated.** `view::preview::entry` sets `GraphLine::ghost_lanes` to the lanes the ghost node itself touches (its own lane, the lanes closing into it, every edge leaving it). A ghost lane that runs *through* a row without touching its node is not in that set, so it would draw solid. With the shapes M5 produces (a replay chain and a ghost merge) the two coincide; a wider fan-out would show the difference. Unverified rather than broken.

**KNOWN_GAPS rows for the conductor** (builders do not edit that file):

- `Esc` = abort a conflicted history operation → M7. Not built here; `HISTORY_CANNOT_STOP` says so in words instead.
- Pixel-tier ghosts → M8. The pixel tier draws the preview at the text tier.
- `(ghost)` badge — needs `BadgeKind::Ghost` in `graft-ui` (above).
- Ghost lane approximation (above) — unverified, not broken.
- `restore` writing nothing when a ref is unreadable (group 06's finding) — untouched here; left **unverified, not broken**.

**Gates**, workspace-wide, from this worktree with `CARGO_TARGET_DIR` outside the shared `target/`:

- `cargo fmt --all --check` — pass
- `cargo clippy --all-targets --message-format=short -- -D warnings` — pass
- `cargo nextest run --workspace` — **2044 passed, 0 failed, 31 skipped** (baseline at `950ab1d` was 2006 passed / 31 skipped, so +38)
- `cargo test --doc` — pass (0 doctests; a gate, not evidence)
- `tests/it/seam_rule.rs` green with the preview read in place: it polices identifiers (`GitInvocation`, `Step::*`, `render_pending`, `render_outcome`, `PathScope::`), not the `graft_ops` module path, so calling `graft_ops::preview::*` from `jobs/preview.rs` needed no new allowance.

## Handoff (pieces 6–10)

Everything below is unstarted unless noted. The seams they attach to are all built and compiling.

1. **Piece 6 — the `.` menu verbs and Fix…** Add the `ContextAction` variants to `graft-ui/src/context_menu.rs` (labels only — the ticket allows that file) and answer them in `update/context_menu.rs` by building the `Operation` and calling `update::preview::open_preview(state, commands, PreviewSubject::Operation { op, opened_by: "." }, false, next_id)`. `open_preview` already refuses correctly while a write runs or git is mid-operation, and already closes the menu's own modality question by being modal itself. Mirror `history_operation` in `update/preview.rs` for how a row's `Source` is read (`source_at`) and how the current branch becomes a `Tip` (`current_branch`).
2. **Piece 7 — `operated` for the history operations.** `jobs::operate::Operated::Conflicted` already exists with `{ description, kind, paths, stalled }`; the ticket's Signatures block wants `entry: EntryId` on it instead. That is a `graft`-side change in `jobs/operate.rs` plus the banner work, which the ticket lists as `app-conflict-interim`'s.
3. **Piece 8 — the journal's `Enter`.** The whole write path is **already built and tested through the type system**: `Command::RevertTo(id, entry, force)`, `Task::RevertTo`, `JobRunner::run_revert_to`, `jobs::undo::run_revert_to`, `Product::RevertedTo`, `update::mod::reverted_to_answer`, `update::undo::reverted_to`, `update::undo::revert_to_forced`, `ConfirmThen::RevertTo`, and `PreviewSubject::RevertTo` with `jobs::preview::run_preview` planning it from `plan_revert_to`. **What is missing is only the key**: in `update/journal.rs`, on `Enter`, call `open_preview(..., PreviewSubject::RevertTo(entry_id), /* return_to_journal */ true, next_id)` and set `state.journal.shown = false` (`close_preview` already puts it back). Also the `OLDER_ENTRY` delete the ticket names. **None of the revert path has been exercised at runtime** — it compiles and its answer-side is unit-tested via `update::undo`, but no test drives a real `revert_to` yet. Treat it as unverified.
4. **Piece 9 — `p` on a diverged branch.** `RemoteState.diverged_upstream` does not exist yet; add it in `update/remote.rs` and have `remote_key(Pull)` open a preview of the merge instead of pulling.
5. **Piece 10 — as the ticket lists it.**
6. **Piece 11's remaining tests.** Written: three in `crates/graft/tests/it/preview.rs`. Still wanted: a stale-tip refusal (`check`'s `Moved` at confirm time), a confirm that actually runs a rebase end to end, and the time machine.
7. **`as-built.md`** — the eight deviations above are written out in this ticket's NOTES ready to be moved across.

---

## NOTES (builder, m5/08 — continuation 1)

**Scout: yes.** A Sonnet `Explore` over `update/journal.rs`, `update/context_menu.rs`
+ `graft-ui/src/context_menu.rs`, `update/remote.rs`, `update/operate.rs` +
`jobs/operate.rs` and `update/undo.rs` before any source was edited (ADR-0035).
It is what established that `ContextAction` has no history variants and that
`Refusal::{Moved, DirtyTree}` were unhandled anywhere in `app`.

**Compaction: no.**

**Piece 6 is `blocked: needs graft-ui`.** The verbs cannot be added from
`crates/graft/` alone: `ContextAction` and `MenuItem` live in
`crates/graft-ui/src/context_menu.rs`, and the ticket's own Signatures block
asks for eleven new variants there. The ticket grants that one file; the
conductor's fence for this continuation is `graft` only, so it stopped at the
seam rather than reaching across. **The signature needed**, verbatim from the
Signatures block:

```rust
// graft-ui/src/context_menu.rs
pub enum ContextAction { …, Checkout(CheckoutTarget), Merge { source: Source }, Rebase { onto: Source },
    CherryPick(Oid), Revert { commit: Oid, mainline: Option<u8> }, Reset { to: Oid, mode: ResetMode },
    DeleteBranch { name: String, at: Oid }, DeleteTag { name: String, at: Oid },
    FixRevert { mainline: u8 }, FixReset { to: Oid }, FixRebase { onto: Oid } }
```

`CheckoutTarget` and `ResetMode` already exist, in `graft_ops::operation::history`
(:177 and :156) — `graft-ui` would have to depend on `graft-ops` to name them,
or the variants carry the plain parts (`String`/`Oid`/a local `ResetMode`) and
`update/context_menu.rs` rebuilds the `Operation`. **That is a design choice
the conductor should make, not the next builder**: `graft-ui` naming
`graft-ops` types is an ARCHITECTURE.md question.

Everything on the `graft` side of piece 6 is otherwise ready: `open_preview`
already refuses correctly, `preview::source_at` and `preview::current_branch`
are the row-reading rules to mirror, and `update::context_menu::dispatch`
already has the `next_id` closure the call needs.

**Mutations run, and what caught each** (a green run is not evidence a test
can fail):

1. `confirm_preview`'s `RevertTo` arm stopped pushing `Command::RevertTo`
   (the write never launches, everything still compiles and every unit test
   still passes). Caught by exactly one test:
   `tests/it/preview.rs::journal_enter_on_an_older_row_previews_and_confirm_really_reverts`.
   730 unit tests passed under the mutation — which is the point: only the
   integration test actually exercises the write.
2. `update::undo::launch` back to `Stoppable::No("MUTANT")`. Caught by
   `update::remote::tests::esc_cancels_an_undo_in_flight`, and by nothing else.
3. `history_refusal`'s `Moved` arm returning `keys: None`. Caught by
   `update::operate::tests::a_refused_history_operation_says_which_key_answers_it`,
   and by nothing else.
4. `remote_failed` no longer recording `diverged_upstream`. Caught by
   `update::remote::tests::diverged_pull_sets_m_r_source`, and by nothing else.

Each mutation was applied, the suite run, the failure read, and the file
restored from a copy taken before the edit.

**Captures** (the **user** judges them; this builder does not tick "looks
right"). New: `docs/shots/m5-ui-preview-time-machine-80x24.txt` (2.2 KB) and
`-160x48.txt` (3.3 KB), written by
`tests/it/preview.rs::capture_time_machine_for_docs_shots` (`#[ignore]`d)
from the real path — a real repository, three real stages through the changes
pane, the real journal, the real `plan_revert_to`, the real frame at the real
size. **Colour is unverified**: a `TestBackend` character grid carries no
colour attributes (CLAUDE.md rule 5). The two rebase captures from the first
half are untouched.

One thing the capture shows that is worth a human eye: the revert box has
**no command lines** under it, because `preview::plan_revert_to` produces no
`commands`. That is `graft_ops`' own answer and the box is honest about it
(every other line is there), but if the user expects a `git …` line per step
the way the rebase box has one, that is a `graft-ops` gap, not an app one.

**What could not be verified:**

- **No test drives a real cancelled undo through the app.** `esc_cancels_an_undo_in_flight`
  proves the key sends `CancelJob`; `Undone::Cancelled`'s two status lines are
  unit-tested through `undone`. Nothing in `crates/graft` runs a real
  `restore::execute`, cancels it, and reads the repository back —
  `graft-ops`' `undo_cancel::*` suite does that below the app. **Unverified,
  not broken.**
- `graft-ops::it::undo_cancel::a_cancelled_undo_through_u_leaves_u_where_it_was`
  **failed once** under a full-workspace run and passed three times in a row
  in isolation afterwards, and passed in every subsequent full run. Nothing
  here touches `graft-ops`. It looks load-sensitive; the group 06 fix round
  should know.
- The `(ghost)` badge is still absent (first half's NOTES); untouched here.

**KNOWN_GAPS rows for the conductor** (builders do not edit that file):

- The first half's five rows still stand, minus nothing.
- `KNOWN_GAPS.md:501` / `:502` ("`Esc` cannot cancel u/U at all", "Proof is
  that `CancelJob` is never sent") — **both closable now.** `UNDO_CANNOT_STOP`
  is gone, `u`/`U` launch `Stoppable::Yes`, and `esc_cancels_an_undo_in_flight`
  is the proof the second row asked for.
- `Esc` = abort a conflicted history operation → M7 (unchanged;
  `HISTORY_CANNOT_STOP` says so in words).
- New: **the revert box carries no command lines** because
  `preview::plan_revert_to` returns none. `graft-ops`, small.
- New: `graft_ops::apply::remote::explain`'s `NotFastForward` string still
  reads "merge or rebase (M5); shell: git pull --rebase". The app now
  overrides it whenever the tracking tip is known, so that text is only
  reachable when it is not. `graft-ops` tidy-up, outside this fence.

**Gates**, workspace-wide, from `.worktrees/m5-08-cont` with
`CARGO_TARGET_DIR=/tmp/graft-m508cont-target`:

- `cargo fmt --all --check` — pass
- `cargo clippy --all-targets --message-format=short -- -D warnings` — pass
- `cargo nextest run --workspace` — **2060 passed, 0 failed, 33 skipped**
  (main was 2054; +6)
- `cargo test --doc` — pass (0 doctests; a gate, not evidence)

## Handoff (continuation 2)

1. **Piece 6**, once the conductor has decided how `graft-ui` names the
   operation types (above). It is the only unbuilt scope item.
2. **Piece 11's remaining tests**, listed under "Not ticked". The
   stale-tip one is the interesting one: `git update-ref` between `Ready`
   and `Enter`, then assert the sticky names `main` and offers `r again` —
   the app half of that wording is built and unit-tested, but nothing drives
   it from a real moved ref.
3. A real cancelled undo through the app, if the conductor wants the
   app-level proof (above, under "could not be verified").


---

## NOTES (builder, m5/08 — continuation 2, piece 6)

**Scout: yes.** A Sonnet `Explore` over `graft-ui/src/context_menu.rs` and
`graft/src/update/context_menu.rs` before any source was edited (ADR-0035).
It is what established that `ContextAction` was `Copy` with eight payload-free
variants, that `MenuItem::label` was `&'static str` (so a dynamic label needed
a type change, not just a new variant), that `ALWAYS_ITEMS` was a `const`, and
that neither file touched `SanitizedText` at all.

**Compaction: no.**

**The `graft-ui` → `graft-ops` edge was not added, and is not needed.**
The conductor's ruling (ARCHITECTURE.md:191) is what this is built to.
`ContextAction` gained **one** variant, not eleven:

```rust
// graft-ui/src/context_menu.rs
pub enum ContextAction { …, History(usize) }
pub struct MenuItem { pub action: ContextAction, pub label: graft_term::SanitizedText }
pub fn for_row(marked: bool, history: Vec<SanitizedText>) -> ContextMenu
```

The widget says **which row of itself was chosen** and nothing more. The
mapping lives in the new pure module `graft/src/update/context_menu/verbs.rs`:
`history_verbs(id, parents, subject, &RefBadges, &[Finding]) -> Vec<HistoryVerb>`,
where `HistoryVerb { label: SanitizedText, op: graft_ops::operation::Operation }`.
`ContextAction::History(i)` means `verbs[i].op`. `CheckoutTarget`, `ResetMode`,
`Source`, `Tip` and `Oid` are named only in `graft`, at the same boundary every
other view model is assembled at. `crates/graft-ui/Cargo.toml` is untouched.

Two consequences worth the conductor knowing:

- Making `label` a `SanitizedText` **tightened** the boundary rather than just
  satisfying it. It is the first menu label that is repo-derived at all, and
  the type now makes it impossible to put an unsanitised one there. `RefName`
  already refuses control bytes, so the reachable attack today is an invisible
  character (a right-to-left override in a branch name) — which is what the
  sanitising test uses, because a fabricated `\x1b` fixture is unreachable and
  would have been a test that cannot fail.
- `AppState::context_menu` is now `Option<update::context_menu::OpenMenu>`
  (`{ row, menu, verbs }`) instead of `Option<(usize, ContextMenu)>`. The verb
  list has to travel with the menu it was built for; two fields kept in sync
  would have been the obvious bug. Four call sites adapted (`mouse.rs`,
  `view/frame.rs`, `update/state.rs`, and the module's own tests).

**Deviations from the ticket's Signatures block** (for `as-built.md`):

9. `ContextAction::History(usize)` replaces the eleven payload-carrying
   variants the Signatures block listed. Reason above — the conductor's ruling.
10. `MenuItem::label` is `SanitizedText`, not `&'static str`; `ALWAYS_ITEMS`
    (a `const`) became `always_items()` (a fn), since only `sanitize_line` can
    construct the type.
11. `ContextMenu::for_row` takes `history: Vec<SanitizedText>`.
12. `apply_context_action` and `context_menu::dispatch` each gained a
    `verbs: &[HistoryVerb]` parameter.
13. `update::preview::{current_branch, source_at, short_name_of}` widened from
    private to `pub(super)` so the verbs reuse the very rules `m`/`r` use
    rather than restating them.

**Two places the ticket's wording was ambiguous. I picked, and here is why.**

1. *"`Merge <b> into <cur>` per local/remote badge (row's source rule)"* — read
   as **one** merge item, with `<b>` chosen by the source rule
   (`preview::source_at`: first local badge, else first remote, else the
   commit). The alternative (one item per badge) would have made Merge plural
   while the ticket's very next clause makes Rebase singular, and the design
   note's own label list is `Merge <branch> into main` / `Rebase main onto
   <branch>` — singular both. It also keeps the menu saying exactly what `m`
   says, which is the menu's stated job. **If the conductor meant one item per
   badge, this is a one-line change in `merge_and_rebase_verbs`.**
2. *"`Fix: rebase instead` → `Rebase { onto: Source::Commit(the other parent) }`"*
   — read as the **non-first** parent, which the finding calls `real_tip`
   ("the default branch's real previous tip, now a non-first parent"). Not
   `first_parent`. This is not only the literal reading of "the other parent"
   against a finding that names `first_parent` explicitly; it is the only one
   that works: `merge-base(merge, real_tip) == real_tip`, so rebasing onto it
   replays the topic commits on top of the branch, which is the merge the user
   meant to make. Onto `first_parent` the merge-base is `first_parent`, so git
   would replay the **default branch** onto the topic — the mistake turned
   around. The other two fixes already aim at `real_tip`, so all three now
   agree on where "before the merge" is.

**Proposed change (not made): the history verbs carry no key in their labels.**
`graft-ui`'s documented invariant is "every label but `Explain`'s names its
key". The ticket's and the design note's label tables give the verbs no key, so
that is what shipped, and the invariant's test now says "every label but
`Explain`'s and the history verbs'". Most of the verbs genuinely own no letter
(keymap §2.1 is why they are behind `.` at all) — but **`Merge` and `Rebase`
do**: they are `m` and `r`. `Merge main into feature (m)` would teach the key
the way `Push (P)` does. That is a label change in one function and a spec
question, so it is here rather than done.

**Not offered, deliberately:** `Reset <cur> to here (soft)`. The design note
and the ticket both list two reset items, mixed and hard.

**Mutations run, and what caught each** (a green run is not evidence a test can
fail; each mutation was applied, the **workspace** suite run, the failures read,
and the file restored from a copy taken before the edit):

1. `ContextMenu::for_row` numbering its `History(i)` by the **menu row**
   instead of the caller's list index (compiles; every verb then means the
   wrong operation, or none). Caught by five:
   `graft_ui::context_menu::tests::a_history_index_names_the_apps_own_list_not_the_menu_row`,
   `graft_ui::context_menu::tests::history_verbs_sit_between_the_rows_actions_and_the_dropped_keys`,
   `update::context_menu::tests::a_branch_row_lists_its_history_verbs_between_its_actions_and_the_dropped_keys`,
   `::a_history_verb_previews_the_operation_its_label_named_and_runs_nothing`
   and `::every_history_verb_the_menu_lists_opens_a_preview`.
2. `fix_verbs` aiming all three Fix items at `first_parent` instead of
   `real_tip` — the reading I rejected above. Caught by three, all in
   `update::context_menu::verbs::tests`:
   `a_foxtrot_merge_head_is_on_offers_all_three_fixes`,
   `fix_rebase_is_absent_unless_head_is_on_the_merge` and
   `no_fix_item_when_the_previous_tip_is_not_a_parent`.
3. `apply_context_action`'s `History` arm returning no command without opening
   a preview (the verb becomes a label that does nothing; everything compiles).
   Caught by four: `update::context_menu::tests::a_history_verb_previews_the_operation_its_label_named_and_runs_nothing`,
   `::every_history_verb_the_menu_lists_opens_a_preview`,
   `::a_verb_chosen_while_a_write_runs_is_refused_in_words` (so the refusal
   test is not vacuous either) and
   `tests/it/preview.rs::choosing_a_menu_verb_previews_that_operation_and_writes_nothing`.
4. `delete_verbs` offering `Delete branch` for the branch `HEAD` is on. Caught
   by exactly one:
   `update::context_menu::verbs::tests::the_current_branchs_own_row_offers_neither_checkout_nor_delete_nor_merge`.
5. `checkout_verbs` building `CheckoutTarget::Branch` from the **short** name
   instead of the full ref — the shape `check_checkout` looks up in the facts,
   so it would have refused `NoSuchRef` at confirm time. Caught by three, all
   in `update::context_menu::verbs::tests`:
   `menu_verbs_build_expected_operations`,
   `a_detached_head_keeps_the_verbs_that_do_not_need_a_branch` and
   `an_invisible_character_in_a_ref_name_is_cleaned_out_of_the_label`.

**Captures** (the **user** judges them; this builder does not tick "looks
right"). New: `docs/shots/m5-ui-context-menu-verbs-80x24.txt` (2.6 KB) and
`-160x48.txt` (3.5 KB), from `.` on `main`'s row of the same `forked()` fixture
the rebase captures use — `feature` checked out, so the menu reads `Checkout
main`, `Merge main into feature`, `Rebase feature onto main`, `Reset feature to
here (mixed)`/`(hard)`, `Delete branch main`, and offers neither `Checkout
feature` nor `Delete branch feature`. **Colour is unverified** (CLAUDE.md rule
5). One thing worth a human eye: the menu is `longest label + 4` wide, and the
history verbs make it about twice as tall as before, so it now covers a good
part of the graph — at 80 columns the graph's `age`/`sha` columns still show to
the right of it. That is the widget's existing anchoring, not new here, but it
is the first time the menu has been this big.

**What could not be verified:**

- **Merge and Rebase from the menu are only verified as far as the picture.**
  They reuse `preview::source_at`, which spells a branch source with its
  **short** name (`feature`), and `graft_ops`' `refs_of_merge`/`source_holds`
  read and check the ref under that same string. Whether `read_facts` resolves
  a short name is a `graft-ops` question I did not open, and no test in the
  workspace confirms a menu-or-key merge/rebase end to end. This is **inherited
  from `m`/`r` (piece 5), not introduced here** — the menu builds the identical
  value, proved by `the_merge_and_rebase_verbs_are_the_ones_m_and_r_build`.
  **Unverified, not broken**, and the same gap on both paths.
- **No verb has been confirmed** — every test stops at the picture, which is
  what `.` is for. The one write path this milestone exercises end to end is
  the journal's time machine (continuation 1).
- The `(ghost)` badge is still absent (first half's NOTES); untouched here.

**KNOWN_GAPS rows for the conductor** (builders do not edit that file):

- Earlier rows all still stand.
- New: **menu/`m`/`r` merge and rebase are unverified past the preview** — no
  test confirms one and reads the repository back (above). Unverified, not
  broken.
- New (small, cosmetic): `Merge`/`Rebase` menu labels do not name their keys
  `m`/`r`, unlike every other keyed label in the menu — see Proposed change.

**Gates**, workspace-wide, from `.worktrees/m5-08-p6` with
`CARGO_TARGET_DIR=/tmp/graft-m508p6-target`:

- `cargo fmt --all --check` — pass
- `cargo clippy --all-targets --message-format=short -- -D warnings` — pass
- `cargo nextest run --workspace` — **2106 passed, 0 failed, 34 skipped**
  (main at `1958cf5` was 2082 passed / 33 skipped, so +24 and one more
  `#[ignore]`d capture test), 49.3 s
- `cargo test --doc` — pass (0 doctests; a gate, not evidence)
