# Scout: the graft app crate's preview, overlay and key-routing surface

Sonnet `Explore`, 2026-09-20, for M5's pass 2 specs. Reports what the code **is**.
Written to this file by the conductor: `Explore` is read-only and could not write it.

## 1. Key routing

`crates/graft/src/update/keys.rs::key_event` is the single entry point (called from
`core_event` in `update/mod.rs` after the credential prompt and search prompt have
first refusal). Fixed precedence, each step returning early if it claims the key:

1. `confirm::confirm_key` (modal, wins even over context menu)
2. `context_menu::context_menu_key` (modal)
3. `commit_editor::editor_key` (modal, claims all but Ctrl+C)
4. `Ctrl+Z` suspend (if `suspend_on_ctrl_z`)
5. `u`/`U`/`Ctrl+Z` undo (blocked while `text_field_open()`)
6. `Esc` → `remote::cancel_operating` (stops a running write)
7. `Esc` → dismiss `Status::Failed`/`Sticky`
8. `pane_and_overlay_key` (`keys.rs:123`): detail pane → map → journal/stash (order
   flips on `detail.focus`) → overview strip
9. `commit_graph::commit_graph_key`
10. `graph_key` — the graph's own keys (last resort)

A mode is entered/left only by flipping a state field to `Some`/`true`; the module's
own `_key` function returns `true` for every key while open, even unused ones ("modal
swallows the keys it does not use"). **There is no generic modal-stack abstraction** —
each overlay hand-writes its own claim/close.

Graph-screen keys claimed in `graph_key` (`keys.rs:161-197`): `q`/`Ctrl+C` quit,
`Ctrl+R` refresh, `F2` screen toggle, `+` lane toggle, `F` focus toggle, `F3` tier
cycle, `y` copy sha, `x` compare, `.` context menu, `c`/`a` commit/amend, `f`/`p`/`P`
fetch/pull/push, `Space`/`1`/`H` view changes, `j/k/g/G`/arrows/PageUp/PageDown motion,
`Esc` (search/walk/focus/topo-sort/cancel-await). `u`/`U`/`Ctrl+Z` (undo), `S` (stash),
`J` (journal), `M` (map) are claimed earlier, in `pane_and_overlay_key`.

## 2. Overlays and modals

| File | Item | Holds / draws |
|---|---|---|
| `update/confirm.rs` | `PendingConfirm` (`AppState.confirm`) | `ConfirmView` (question/consequence/command) + `ConfirmThen::{Operate(Operation), Undo{redo}}`. Opened by `d` (discard), stash drop, force-push lease box. Closed on any key but Left/Right/Tab/BackTab (move cursor) / Enter (dispatch). |
| `update/credential.rs` | `CredentialPrompt` (`AppState.credential`) | title, sanitized prompt text, `TextField`, `masked` flag, owning `JobId`. Opened by `Progress::CredentialNeeded`; closed by Enter (`Command::AnswerCredential`) or Esc (cancels the write). Auto-closed if orphaned (`close_if_orphaned`, every `update`). Drawn over every screen. |
| `update/context_menu.rs` | `(usize row, ContextMenu)` (`AppState.context_menu`) | row index (or `WORKING_TREE_ROW` sentinel) + items (CopySha/CompareWithThis/OpenDetails/Explain). Opened by `.`/right-click; closed by any key, dispatched on Enter. |
| `update/stash.rs` | `StashState` (`AppState.stash`) | `shown`, `selected`, `entries: Loading<Arc<Vec<StashEntry>>>`, `preview: Loading<Arc<FileListView>>`, `prompt: Option<StashPrompt>`. Opened by `S` (graph focus only); drawn by `view/stash.rs::stash_view`. Save prompt is modal within it. |
| `update/journal.rs` | `JournalState` (`AppState.journal`) | `shown`, `selected`, `expanded`, `notice_seen`, `entries: Loading<Arc<JournalRead>>`, `reading: Option<JobId>`, `unreadable: Vec<String>`. Opened by `J`. **`Enter` on an older-than-next-undo row is refused today** — it only sets `Status::Done(OLDER_ENTRY)` naming Ghost Preview (M5). |
| `update/map.rs` | `MapState` (`AppState.map`) | `shown`, `data: Loading<Arc<MapData>>`, `progress: Option<(done,total)>`, `selected`. Opened by `M`. Rebuilt from scratch on open/resize, discarded on close (never cached). |
| `update/commit_editor.rs` | `CommitEditor` (`AppState.commit_editor`, plus `commit_draft`) | Opened by `c`/`a`; modal, claims every key but Ctrl+C; drawn via `draw_commit_editor` in `view/frame.rs`. |

Draw dispatch (`view/frame.rs::view_with_graphics`, lines 198-260): a strict
`match state.screen` with guard arms — commit editor > Map > Journal > Stash > plain
graph. Only one full-screen overlay renders at a time; context menu, confirm box and
credential prompt draw as layers **on top**, each an independent `Option` check.

## 3. Jobs

Launch: `update::operate::launch` (`update/operate.rs:56`) mints a `JobId`, sets
`AppState.operating = Some(id)` and `Status::Busy`, calls `remote::launched(...,
Stoppable::Yes)` to record a `Running` entry, and pushes `Command::Operate`. Every
write site goes through this one function or its undo twin (`undo.rs:66`, which passes
`Stoppable::No(UNDO_CANNOT_STOP)`).

Runner: `jobs/runner.rs::JobRunner::dispatch` → `self.operate(id, op)` (`runner.rs:546`)
→ `jobs::operate::run_operation` (`jobs/operate.rs:159`) → `graft_ops::apply::apply`.
Result returns as one `Product::Operated`, handled by `update::operate::operated`
(`operate.rs:645`). Pool is 4 workers (`jobs/pool.rs`).

Progress kinds: `Progress::Operating{description}`, `Progress::RemoteProgress{brief}`
(remote only), `Progress::CredentialNeeded{prompt}`. Completion is exactly one of
`JobFinished`/`JobFailed`/`JobCancelled`, or `Product::Operated` for operate jobs.

`Stoppable` (`update/remote.rs:70`): `Yes` or `No(&'static str)` on the `Running`
entry. **Binary and static** — set once at launch, never updated mid-flight, consulted
only by `stop_write`/`cancel_operating` (Esc). The executor checks the stop token
**once, before the first step** (`UNDO_CANNOT_STOP`), so a stopped multi-step write can
leave a plan half-applied with no rollback. No job reports step-level progress for
cancellation.

Cancel path: `Esc` → `remote::cancel_operating`/`stop_write` → `Command::CancelJob(id)`
→ `JobRunner::cancel`/`Live::cancel` → token → `Outcome::Cancelled` →
`Event::JobCancelled` → `update::mod.rs::job_cancelled` → `operate::write_job_ended`.

## 4. Refresh

`repository_opened` (`update/mod.rs:314`) sets `AppState.repository`, clears a stale
`Status::Failed`, calls `commit_graph::decide_commit_graph_offer`. The real refresh is
`keys::refresh_key` (`update/keys.rs:263`, reused by `operate::refresh_scope`'s
`Refresh::Everything` and `Ctrl+R`): re-reads journal, stash (if shown), detail pane,
cancels `awaiting`/`promoting` walks, issues `OpenRepository` + `ReadHistory` +
`ReadRefs` + `ReadWorkingTree` + remote request, plus `radar::reset_for_refresh`.

Survives refresh: `history.view` (fold/first-parent/hidden-stashes) and the selected
commit (`reanchor`), carried into the new `History` in `history_started`
(`update/mod.rs:333-344`). Overlay states (`map`, `journal.shown`, `stash.shown`,
`confirm`, `context_menu`, `credential`, `commit_editor`) are untouched — an open
overlay stays open across `Ctrl+R`, re-reading only its own list.

## 5. Where `Operation`, preview and `apply` meet the app

`graft_ops::operation::Operation` (`crates/graft-ops/src/operation.rs:35`) has:
`Stage`, `Unstage`, `Discard`, `Commit`, `Amend`, `StashPush/Apply/Pop/Drop`, `Fetch`,
`Pull`, `Push`. **No `Merge`/`Rebase`/`Reset`/`CherryPick`/`Revert` variants exist.**

Call sites building an `Operation`:
- `update/operate.rs`: `stage_selected`, `stage_at_diff_cursor`, `request_discard`,
  `request_discard_at_diff_cursor`, `stage_all` → `launch(...)` or a `PendingConfirm`.
- `update/stash.rs`: `save` (`StashPush`), `operate_on_selected` (`StashApply`/`Pop`),
  `request_drop` (confirm → `StashDrop`).
- `update/confirm.rs::confirm_key`: the one place `ConfirmThen::Operate(op)` becomes
  `operate::launch`.
- `update/remote.rs`: fetch/pull/push.
- `update/undo.rs`: **not** an `Operation` — a separate `Command::Undo`/`Redo` path
  through `jobs::undo::run_undo`, because it plans from the journal, not from a key.

`apply` call site: exactly one — `jobs/operate.rs::run_operation` (line 197), from
`JobRunner::operate`, from `Command::Operate`. Nothing else in `crates/graft` calls it.

**Preview today** is only: the stash overlay's file-list preview (`Space`, a `ListFiles`
read) and the detail pane's diff. There is **no** preview of an `Operation`'s effect —
the confirm box shows a hand-built question, prose consequence and literal shell command
per call site (`discard_command`, `discard_patch_command`), not derived from the
`Operation`. "Ghost Preview" appears only in comments and ADRs (`update/journal.rs:20-22,217`,
`graft-ops/src/lib.rs:3`, ADR-0031, ADR-0032) as an unbuilt M5 concept.
