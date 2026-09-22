# Scout — g05 app-resolver-whole-files-and-base

Explore, sonnet, 54 tool uses, 210 s, 2026-09-22. Read-only; conductor wrote this file from the reply. Facts with lines as reported by the scout; the keymap claim was checked by the conductor.

## Spec gate: already satisfied (conductor-verified)
`docs/design/02-keymap.md:245-246` already carries `B` (show/hide BASE) and `d` (start this file over, behind a confirm box), commit `4dfa409`. The ticket's line-13 fallback (`blocked: keymap`, ship pieces 1 and 4 only) **does not apply**. Build all four pieces.

## ResolverState — `crates/graft/src/update/resolver.rs:48-78`
- Fields: `path`, `opened_from`, `file: Option<Arc<ConflictFile>>`, `cursor_block`, `top`, `base_shown: bool` (:67, exists, always `false` today), `job: Option<JobId>`, `staging: Option<JobId>`, `notice: Option<(String, bool)>`.
- **One shared `job` slot** for open-reads and takes. A `LoadBase` job contends for it — decide explicitly (new field, or prove base-load and take cannot race).
- `act()` :172-201: `o`/`t`/`b` take, `n`/`N`, `Enter`, `e`, `M`, default scroll. **No `B` or `d` arm.**
- `conflict_arrived` :509-529: `file: None` (whole-file/binary) **closes the resolver** with a Sticky status `"<path>: <words> — binary and whole-file conflicts"`, keys `M mergetool  e edit`. This is the branch piece 1 replaces.

## View
- `view/resolver.rs:24-26` `resolver_view()` returns `None` when `open.file` is `None` → the widget's existing `RegionView::WholeFile` is unreached from the app.
- `DocView.base_shown` passes through `open.base_shown` (:45). `BlockView.base` and `Block.shown_base()` (graft-git `conflict.rs:171`, used at view :209-221) are render-ready; nothing ever attaches a base, so it is always `None`.
- Hint bar: `view/rows.rs:399` always builds `HintContext::Resolver { base: false, restart: false }`; `graft-ui/src/graph_screen/hints.rs:905-921` draws **no** `B`/`d` hint when false. Going live = pass `true` from `rows.rs` once bound.

## Confirm boxes — `crates/graft/src/update/confirm.rs`
- `PendingConfirm { view, then: ConfirmThen }` :18-23. `ConfirmThen` :31-75: `Operate`, `Undo`, `RevertTo`, `Resume`, `UndoAfterPush` — **no `RestartConflict`**.
- `confirm_key` Enter :100-135 flat match on `then`; the `Operate(op)` arm :111-113 calls `super::operate::launch` — the new arm mirrors it.
- Copy: `abort_pressed`, `update/banner.rs:426-441` (`ConfirmView::new(title, lines, command).with_labels("Cancel","Abort")`, all text through `sanitize_line`); `skip_pressed` :398-424 same shape. `ConfirmView` API: `graft-ui/src/confirm.rs:76,91,102`.

## Jobs — pattern for `LoadBase`
- `jobs/conflict.rs`: `open()` :75-95, `take()` :112-129 — open repo, call `graft_ops::conflict`, map errors to app-level structs (ops errors are not `PartialEq`).
- `graft_ops::conflict::base_for(repo: &RepoHandle, path: &Path, file: &mut ConflictFile) -> Result<usize, BaseError>` (`graft-ops/src/conflict.rs:188-196`) — needs the repo, not a pure function over the file. `BaseError { NoBase, Mismatch(..), Diff3 }` :165-181 = the ticket's three notices.
- Runner: `Task::OpenConflict`/`TakeSide` `events.rs:184,187`; `runner.rs:542-579` `open_conflict`/`take_side` via `queue_as`; dispatch `runner.rs:943-944`. `Product::ConflictOpened`/`Taken` `events.rs:~317-330`.
- `update/mod.rs` needs matching arms at ~227-230 (`AppEvent::Produced`), ~385-386 (`Task::OpenConflict | Task::TakeSide` grouping), ~584-585 (catch-all).

## ops surface (built by G1/G3)
- **Tuple variants over param structs**, not the ticket prose's struct-variant shorthand: `Operation::ResolveWholeFile(ResolveWholeParams { path, side, how })`, `Operation::RestartConflict(RestartParams { path })` — `operation.rs:116,119`, structs `operation/conflict.rs:79-93`.
- `Side`, per-`UnmergedHow` table: `operation/conflict.rs:1-38,49-124`.
- `Refusal::NotMarkerShape` (`operation.rs:463-468`, message :617-621) — `check_restart` :238-256, already wired. `Refusal::NotUnmerged` → `"<path> is not conflicted"`.
- `UnmergedHow::words()`/`has_markers()` `graft-git/src/status.rs:192-210`; markers only for `BothModified`/`BothAdded`.

## Stash-pop conflict fixture
No `graft_fixtures::Shape` builds one. Only builder: `about_to_pop_a_conflict()`, `crates/graft-ops/tests/it/unmerged_index.rs:44-57` — raw git, one `UU` path `p.txt`. **Conductor ruling:** build it in-test in `crates/graft/tests/it/` the same way (raw git); no new `Shape` — `graft-fixtures` is outside this group's crate set.
