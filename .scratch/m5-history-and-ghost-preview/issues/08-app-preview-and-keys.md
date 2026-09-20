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

- [ ] Every test in piece 11 green, named at tick time; no `sleep`/poll loop in any of them
- [ ] The `Operation` that runs is the one previewed: `m_opens_preview_and_enter_runs_the_pinned_op`; a moved tip never runs anything: `stale_tip_is_refused_in_words`
- [ ] Every string in the box comes from `PreviewPlan` (`preview_box_lines_come_from_plan`); `rg -n '"git ' crates/graft/src/update/preview*` finds no hand-written git command — inspection, listed in NOTES
- [ ] `preview_key` sits between `confirm_key` and `context_menu_key` in `key_event`; a `u` typed while the preview is open does nothing (`preview_is_modal`)
- [ ] Captures `docs/shots/m5-ui-preview-rebase-{80x24,160x48}.txt` exist, ≤ 8 KB, show `(ghost)` rows with `—` sha and the docked box with `[ Cancel ]` first; the user judges them
- [ ] MAP.md + `as-built.md` updated; NOTES: scout used or why not; turn count at each piece boundary; KNOWN_GAPS rows for the conductor (Esc = abort → M7; pixel-tier ghosts → M8); colour unverified

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft -p graft-ui` · `cargo test --doc -p graft`; `tests/it/seam_rule.rs` still green with the preview read allowed.

## Size

One context, the largest app group. Stop seam after piece 5 (preview core + `m`/`r`), pieces 6–10 in a handoff note. Files: `update/preview*.rs` (new), `jobs/preview.rs` (new), `update/keys.rs`, `update/context_menu.rs`, `update/journal.rs`, `update/operate.rs`, `update/remote.rs`, `update/mod.rs`, `view/rows.rs`, `view/frame.rs`, `jobs/undo.rs`, `jobs/operate.rs`, `jobs/runner.rs`, graft-ui `context_menu.rs`. Runs alone in graft after G1–G3 and G6–G7 merge; the next two app groups build on it.

## Conductor: user conditions (ADR-0038 §5) — binding

- A preview **never loads more history mid-preview**. Parents outside the laid-out slice are clipped and drawn as anchors.
- **If the part of the preview that matters falls outside the slice, the preview says so** — a plain line the user can read — rather than drawing a truncated shape they could misread. Name the test that proves this.
