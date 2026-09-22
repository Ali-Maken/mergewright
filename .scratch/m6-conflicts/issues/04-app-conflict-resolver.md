# app-conflict-resolver: the resolver as a modal overlay — open from a CONFLICTS row or the focused banner, block takes on a job, n/N across blocks and files, ⏎ marks resolved and moves on, e/M hand over and re-read, Esc closes, mouse; the changes-pane hints go live in the binding commit

Status: ready
Blocked by: **01**, **02**, **03**.
Build with: (blank — brownfield across `update/keys.rs` (key chain order), `update/banner.rs`, `update/detail/keys.rs` (`⏎`/`e`/`M`/`space` on a CONFLICTS row), `update/mod.rs` (`AppState`, `Command`, `Product`, `GraphOverlay`), `jobs/`, `event_loop/outside.rs`, `view/`; recommendation: **Opus** — a new modal state machine threaded through the key chain, the overlay enum and the job loop; wrong ordering (resolver vs banner vs confirm box) is structural but the walk and the unit tests catch it, so not Fable.)
Crates: graft, graft-ui (`graph_screen/hints.rs` only, in the one commit that binds `⏎`/`M`/`space` — after G3 has merged, never concurrently with it)
Needs:
- `ops-conflict-operations` — `conflict::{open_conflict, apply_take, mark_resolved_refusal, Opened, OpenError, TakeError}`, `Refusal::{NotUnmerged, ConflictsOpen}`.
- `git-conflict-model` — `conflict::{ConflictFile, Take, BlockAt, BlockState}`, `status::UnmergedHow`, `Shape::{StoppedRebase, StoppedMerge}`.
- `ui-conflict-resolver` — `conflict_resolver::{DocView, RegionView, BlockView, ScreenRow, Layout, layout, row_at, ResolverScreen, SIDE_BY_SIDE_MIN, HEADER_ROWS}`, `HintContext::Resolver`.

**Spec change required before the build** (CLAUDE.md: a key the keymap does not name is not free): `docs/design/02-keymap.md` does not name a key that opens the resolver. The design note proposes (Proposed change 2a) `⏎` on a `CONFLICTS` row (§2.4) and `⏎` on the focused banner (§2.6a) — the global "open / drill in" of §2.2, no new letter — plus `Esc` in the resolver = close (Proposed change 6) and the §2.1 sentence for `M` (Proposed change 3). The conductor edits the doc first; the builder binds nothing the doc does not name at build time. `B` and `d` are **not** this group's (G5).

## Scout first (Sonnet `Explore`, ≤ 1 page) — `.scratch/m5-history-and-ghost-preview/scout-app.md` §1–§4 first; ask only what it lacks

- `crates/graft/src/update/keys.rs`: the exact key-chain order today (`text_field_open` → confirm → commit editor → `preview_key` → `banner_key` → `context_menu_key` → overlays → pane…) and how a modal overlay (`Map`, `Journal`, `Stash`) claims every key; `graph_overlay()`'s variants and where the view chooses what to draw from it.
- `crates/graft/src/update/detail/keys.rs`: how `e`/`M`/`space` on a `CONFLICTS` row find the row's path (`cursor_conflict`?) and what `⏎` does on a file row today (opens the preview? nothing?) — `⏎` must keep that meaning on non-CONFLICTS rows.
- `crates/graft/src/update/banner.rs`: `banner_key`'s full match; `take_focus`/`resume`/`resumed`; how `Operated::Conflicted` lands `banner_focus` (the resolver must open *from* there, not replace it).
- `crates/graft/src/event_loop/outside.rs` + `running.rs`: `Handover::{mergetool, edit_conflict}` and what happens after `Product::HandedOver` (which read is scheduled — the resolver must re-read its file on that same event).
- `crates/graft/src/jobs/`: how a read job with a typed answer is declared (`kinds.rs` / `runner.rs`), so `OpenConflict`/`TakeSide` follow the pattern; whether jobs hold a `RepoHandle` or open one.
- `crates/graft/src/view/`: how `Map`/`Journal` overlays are drawn full-frame under the header, and where `draw_tab_banner` keeps the banner word in the header (the resolver keeps it).
- Mouse: how overlays hit-test today (m5 reviews 10-30-31 findings) — click row → the same function a key calls.

## Read first

- `docs/design-notes/m6-conflicts.md` — Decision 1, Decision 2 (a) (the relaunch lands on the conflict — **G6**, not here, but the opener it needs is here), Interfaces (the `graft` block), Hard-to-reverse choice 2, New UI (keys, empty/error states — the acceptance), Proposed changes 2a/3/4/6.
- `docs/design/02-keymap.md` §2.1, §2.2 (`⏎`, `Esc`, `Ctrl+C`), §2.4, §2.6, §2.6a — after the conductor's edit.
- `docs/design/06-undo-journal-design.md` §6.4 (a take is a step in the live span; `u` is refused while live — unchanged).
- `docs/design/08-architecture.md` §8.2 (every byte from `ConflictFile` becomes `SanitizedText` in `view::resolver` and nowhere else).
- `crates/graft/MAP.md` §update (`AppState`, `Command`, `Product`, `Status`, `GraphOverlay`), §update::banner, §update::detail, §update::preview (the chain neighbour), §view, §event_loop::outside, §jobs; `crates/graft-ops/MAP.md` §conflict (after G2); `crates/graft-ui/MAP.md` §conflict_resolver, §hints (after G3); `crates/graft-git/MAP.md` §conflict.
- `docs/reviews/m5-09-app-conflict-interim.md` (M1 Tab cycle, M2 dead keys, the gaps table); `docs/reviews/m5-10-30-31.md` (hit-test/`answered()` findings — do not repeat them).
- `KNOWN_GAPS.md` rows 235, 558, 229 (a conflicted stash pop has no banner — the resolver opens from the pane and `⏎` at the end closes to the pane), 561 (banner `Esc` — untouched).

## Scope

In, in order (a commit per piece):

1. **State and jobs** — `AppState::resolver: Option<ResolverState>`; `GraphOverlay::Resolver` (drawn over graph and pane; header kept; `text_field_open()`/`graph_has_focus()` answer as for `Map`). `Command::OpenConflict(JobId, PathBuf)` → job calls `open_conflict` against the latest `WorkingTreeStatus` → `Product::ConflictOpened(Result<Arc<Opened>, String>)`. `Command::TakeSide(JobId, TakeRequest)` → `apply_take` → `Product::Taken(Result<Arc<ConflictFile>, String>)`. Errors carry `graft_ops`' words (`OpenError`/`TakeError` `Display`), never `{:?}`.
2. **Opening** — `open_resolver(state, commands, path, from, next_id)`: refuses in words when `state.resolver` is `Some` (never two), when a confirm box or the commit editor is open (`finish the box first`), when a job is busy on that path; else sets `ResolverState { path, opened_from, doc: Loading, .. }` and pushes `OpenConflict`. Bound to `⏎`: in `detail/keys.rs` on a `CONFLICTS` row (`from: Pane`); in `banner_key` on the focused banner (`from: Banner`, path = `first_conflicted(state)`), both **only after the keymap names them**. `ConflictOpened(Ok)`: `file = opened.file`, `cursor_block` = first open block (or `None`), `top = 0`; the notice for the empty states (`no conflict markers · ⏎ marks it resolved`; `too large to show (9 MB) · e edit  M mergetool`; binary/whole-file → G5 — here they show the file card **without** `o`/`t` acting: `Status::Sticky("deleted by them · o/t come with the next build")` is **not** acceptable; instead this group's `open_resolver` refuses `binary and whole-file conflicts: M mergetool  e edit` for `file: None` until G5 lands — in words, no dead screen). `ConflictOpened(Err)` → `resolver = None`, `Status::Sticky { reason: <words>, keys: Some("e edit  M mergetool") }`.
3. **`resolver_key`** — chain position: right after `preview_key`, before `banner_key`; modal: claims every key but `Ctrl+C` while `state.resolver.is_some()`. `o`/`t`/`b` → `TakeSide { path, expected: file.fingerprint, block: cursor_block, take }` (refused in words when `cursor_block` is `None`: `no conflict under the cursor · n next`); `Taken(Ok(file))` → replace `file`, keep `cursor_block`, **scroll so the block's header is in view** (`layout.block_row[block]` vs `top`/height), `Status::Done("took ours in <path> · 2 of 3 open")`; `Taken(Err)` → notice `(words, true)`; a `Changed` error → also re-push `OpenConflict` (re-read; the notice reads `<file> changed on disk · re-read`). `n`/`N` → next/previous **open** block; past the last open block of this file → the next conflicted file in `unmerged_paths(state)` order (closes and reopens with `from` kept), at the very end `Status::Sticky("no more conflicts · ⏎ marks <file> resolved")`. `⏎` → `mark_resolved_refusal` first (`2 conflicts still open in <path> · n next`, no job), else `Operation::Stage(Target::Paths([path]))` via the existing `operate` path; on the `Operated::Done` for it: advance to the next conflicted file, or — none left — close to `opened_from` with `Status::Done("all conflicts resolved · c continue")` when a banner shows, `("all conflicts resolved")` otherwise (row 229's stash pop). `e` → `OpenExternalEditor { path, line: blocks()[cursor].first_line }`; `M` → `OpenMergetool { path }` (its `None`-mergetool refusal reused); after `Product::HandedOver` the resolver pushes `OpenConflict` for its path again (the file may be resolved or gone from unmerged: `NotUnmerged` → close to `opened_from` with `Status::Done("<path> resolved outside · ⏎ …")`). `j k ↑ ↓ PgUp PgDn g G` scroll `top` within `layout.rows.len() − height`; `Esc` → `resolver = None`, focus back to `opened_from` (banner focus restored when `Banner`). `Tab` → swallowed (modal). Every other key swallowed.
4. **Mouse** — wheel scrolls; click on a row: `row_at(layout, top, y − HEADER_ROWS)` → its `block` becomes `cursor_block`; click on the header rows does nothing. Same functions as the keys (no second code path).
5. **View** — `view::resolver::resolver_view(state) -> Option<DocView>`: the one sanitisation site (`sanitize_line` per line, `sanitize_path` for the title, labels via `sanitize_line`); title `<path> · file i of n · open of total conflicts open` (short form under `SIDE_BY_SIDE_MIN`); explainer when `in_progress.kind == Rebase`: `ours = <onto>, what you are rebasing onto · theirs = the commit being replayed` (`onto` from `InProgressDetail`); for a merge: `ours = <head_name> · theirs = <merging branch>`; `cursor_block`, `base_shown: false` (G5), `notice`. `layout` is computed once per frame from `columns` and cached on `ResolverState` keyed by `(fingerprint, columns, base_shown)` so keys and the widget see the same rows. `hint_context` → `HintContext::Resolver { base: false, restart: false }` while open. Frame `< 60×10` → no key but `Esc`/`Ctrl+C` acts.
6. **Hints go live** (`graft-ui/src/graph_screen/hints.rs`, same commit as piece 2's binding): `⏎ resolve`, `M mergetool`, `space resolved` on `ChangesPane { unmerged }` and `⏎ resolve` on `Banner { conflicted }` flip `planned()` → live; `every_planned_hint_is_still_unbound` passes because they are now bound; `every_key_the_bar_draws_is_answered` covers the resolver context through `bound_actions()`. Also `FileStatusView::Unmerged { how }` now takes `UnmergedHow::words()` (row 472 closes at this seam).
7. `crates/graft/MAP.md` (§update::resolver, §view::resolver, §jobs), NOTES.

Out: `B` base and `d` start over, whole-file/binary `o`/`t` (G5); relaunch-lands-on-the-conflict, `u` after continue, stale lease (G6); `docs/shots/` captures and TESTING.md (G7); the three-column layout; colour.

## Signatures

```rust
// graft/src/update/resolver.rs
pub struct ResolverState { pub path: PathBuf, pub opened_from: OpenedFrom, pub doc: Loading<Arc<Opened>>, pub file: Option<ConflictFile>, pub cursor_block: Option<usize>, pub top: usize, pub base_shown: bool, pub job: Option<JobId>, pub notice: Option<(String, bool)>, pub layout: Option<(Oid, u16, bool, Layout)> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum OpenedFrom { Banner, Pane }
pub fn open_resolver(state: &mut AppState, commands: &mut Vec<Command>, path: PathBuf, from: OpenedFrom, next_id: &mut JobId) -> bool;
pub fn resolver_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyEvent, next_id: &mut JobId) -> bool;
pub fn resolver_mouse(state: &mut AppState, commands: &mut Vec<Command>, event: MouseEvent, frame: Rect) -> bool;
pub fn conflict_opened(state: &mut AppState, commands: &mut Vec<Command>, id: JobId, result: Result<Arc<Opened>, String>, next_id: &mut JobId);
pub fn taken(state: &mut AppState, commands: &mut Vec<Command>, id: JobId, result: Result<Arc<ConflictFile>, String>, next_id: &mut JobId);
pub fn handed_back(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut JobId);   // after Product::HandedOver: re-read the open file
pub fn resolved_advance(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut JobId);   // after the Stage for the open file lands
pub fn next_open_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize>;
pub fn scroll_block_into_view(layout: &Layout, block: usize, top: usize, height: u16) -> usize;

// graft/src/update/mod.rs
pub enum GraphOverlay { /* existing */ Resolver }
pub enum Command { /* existing */ OpenConflict(JobId, PathBuf), TakeSide(JobId, TakeRequest) }
pub struct TakeRequest { pub path: PathBuf, pub expected: Oid, pub block: usize, pub take: Take }
pub enum Product { /* existing */ ConflictOpened(JobId, Result<Arc<Opened>, String>), Taken(JobId, Result<Arc<ConflictFile>, String>) }

// graft/src/view/resolver.rs
pub fn resolver_view(state: &AppState, columns: u16) -> Option<DocView>;
pub fn resolver_title(path: &Path, file_index: usize, files: usize, open: usize, total: usize, wide: bool) -> String;
pub fn resolver_explainer(detail: &InProgressDetail) -> Option<String>;
```

## Acceptance

Every box names its test (`crates/graft/tests/it/resolver.rs` unless said; fixtures `StoppedRebase`/`StoppedMerge`).

- [ ] `⏎` on the `CONFLICTS` row `both.txt` → `Command::OpenConflict`; `ConflictOpened(Ok)` → `graph_overlay() == Some(Resolver)`, `cursor_block == Some(0)`, `hint_context() == Resolver { base: false, restart: false }` — `enter_on_conflicts_row_opens_the_resolver`.
- [ ] `⏎` on the focused banner opens the first conflicted file with `opened_from: Banner`; `Esc` closes and `banner_focus` is true again — `enter_on_banner_opens_and_esc_returns`.
- [ ] `⏎` on a non-CONFLICTS file row keeps its previous meaning (assert against the scout's finding) — `enter_elsewhere_unchanged`.
- [ ] `o` → `TakeSide { expected: <fingerprint>, block: 0, take: Ours }`; `Taken(Ok)` → the file on disk equals `git checkout --ours` bytes, `open()` drops by one, the status line reads `took ours in both.txt · 0 of 1 open` — `o_takes_ours_on_disk`. Same for `t`, `b` — `t_and_b_take`.
- [ ] `three.txt`: `o` on block 0, `n` → cursor 1, `n` → 2, `n` → `no more conflicts · ⏎ marks three.txt resolved` (cursor stays 2); `N` twice → cursor 1 (block 0 is skipped: taken) — `n_and_N_walk_open_blocks_only`.
- [ ] After a take the block's header row is within `[top, top + height)` — `take_scrolls_the_block_into_view` (uses `scroll_block_into_view` on a doc where block 2 starts at row 90 with height 20).
- [ ] `⏎` with two open blocks → `Sticky("2 conflicts still open in three.txt · n next")`, no `Command::Operate` pushed — `enter_refuses_while_open`; after all takes `⏎` → `Operate(Stage)` and, when `Operated::Done` lands, the resolver shows the next conflicted file (`crlf.txt` in status order) — `enter_stages_and_advances`; on the last file → `resolver == None`, `Status::Done("all conflicts resolved · c continue")` with a banner, `("all conflicts resolved")` without one (stash-pop conflict fixture from m5, row 229) — `last_file_closes_to_where_it_opened`.
- [ ] `e` → `OpenExternalEditor { path, line: 3 }` (the cursor block's first marker line); `HandedOver` → `OpenConflict` pushed again; a re-read returning `NotUnmerged` closes to `opened_from` with `Status::Done("both.txt resolved outside")` — `e_hands_over_and_rereads`, `resolved_outside_closes`. `M` the same with `OpenMergetool` and the `no merge.tool configured · e edits <path> instead` refusal reused — `M_hands_over`.
- [ ] `Taken(Err("both.txt changed on disk"))` from a `Changed` → notice set and `OpenConflict` pushed once — `changed_on_disk_rereads_once`.
- [ ] While the resolver is open: `u`, `Tab`, `/`, `.`, `c`, `a`, `s` are swallowed (no command, no focus change); `Ctrl+C` still quits — `resolver_is_modal`.
- [ ] Opening refused in words when a confirm box is open, when the resolver is already open, when the path is not unmerged (`not conflicted`) — `open_refusals_in_words`.
- [ ] Binary/whole-file (`pic.bin`, `theirs-deleted.txt`) → refusal `binary and whole-file conflicts: M mergetool  e edit` until G5 (the test is rewritten there) — `whole_file_refused_in_words_for_now`.
- [ ] `resolver_view` sanitises: a line containing `\x1b[31m` and a path with a newline render with the escapes stripped/replaced — `resolver_view_sanitises`; explainer for a rebase reads `ours = main, what you are rebasing onto · theirs = the commit being replayed` — `explainer_names_the_sides`.
- [ ] Click on `Pair { block: 1 }` row → `cursor_block == Some(1)`; wheel down → `top += 3` (or whatever the scout says the Map overlay uses — same constant) — `mouse_uses_the_key_functions`.
- [ ] `hints.rs`: `every_planned_hint_is_still_unbound` passes with `⏎ resolve`/`M`/`space` live; `every_key_the_bar_draws_is_answered` passes for `Resolver` (graft-ui); the app-level `every_hint_is_answered_by_the_chain` (or the m5 equivalent the scout names) covers `Resolver` — named in the report.
- [ ] Frame 50×8: only `Esc`/`Ctrl+C` act; `o` is swallowed with no command — `tiny_frame_is_inert`.
- [ ] MAP.md updated; NOTES says which scout facts were used and where `⏎` was bound.

## Gates

fmt / clippy / nextest / doc, workspace-wide. `tests/it/seam_rule.rs` still passes (the app never calls `graft_git::conflict::write_back`).

## Size

One context, but the largest of the milestone — ~900 lines. Stop seam after piece 3 if compaction looms (say so; the conductor splits 4–7).
