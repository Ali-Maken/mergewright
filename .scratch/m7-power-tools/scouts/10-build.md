# Scout 10-build: template for `GraphOverlay::Picker` (paths under crates/graft/src unless named)

## Contradictions with the ask / ticket 10
- **C1 Stash overlay is not modal; the picker must be.** Keymap `docs/design/02-keymap.md:75-77` and `:397` say the picker is a **text field that claims every key but Ctrl+C**. Stash/journal/Map are explicitly *not* in `modal_owns_keys` (`update/state.rs:800-802`). The closer key template is the stash **save prompt** (`update/stash.rs:158-164`) plus `text_field_open` (`state.rs:775-786`).
- **C2 `land_on_result` is not a function.** Nothing in `crates/` defines it. Ticket 03 shipped it as the field `AppState::lands_on_head: Option<JobId>` (`state.rs:345`). It is set in `operate::launch*` from `op.moves_head()` (`update/operate.rs:141`) and spent by `operate.rs:1054` → `landed` → `banner::finished_lands_on_head` (`update/banner/stop.rs:91`) → `land_on_head` (`stop.rs:144`, `History::seek(refs.head())`). A confirmed checkout lands with no extra call.
- **C3 `seek_selected` cannot be reused as it stands.** It is private and tied to `MapState`: `fn seek_selected(next, commands)` (`update/map.rs:190`) reads `next.map.selected_tip()` and calls `next.map.close()`. Only its body is generic: `history.seek(oid, graph_rows())` (`history/scroll/query.rs:71`) → `Command::ShowRows(awaiting, req)` (`map.rs:196-204`). Ticket line 131 says "reused, not changed".
- **C4 `switch_to` cannot ask for Esc-back.** `switch_into` hard-codes `return_to_journal = false` (`update/switch.rs:88-95`), and `return_to_journal` restores only `journal.shown` (`preview.rs:694`, `:808`).
- **C5 `graft_ops::switch_list` signature.** Code: `switch_list(repo: &RepoHandle, stop: &dyn Stop) -> Result<SwitchList, FactsError>` (`graft-ops/src/switch_list.rs:84`). Ticket `:87` says `(repo: &Repo, git: &Git, stop)`. Your ask matches the code.

## 1. Stash template (state, open, job, routing, cancel)
- State: `StashState { shown, selected, entries: Loading<Arc<Vec<_>>>, preview, prompt }` (`update/stash.rs:39-51`). `read()` takes a fresh id, sets `Loading::Pending(id)` and returns `Command::ReadStashes(id)` (`:56-60`).
- Open: `S` in `stash_key` (`stash.rs:175-183`): sets graph focus and `shown`, resets fields, then `read`. Close: `close()` (`:311-320`) pushes `Command::CancelJob(id)` if the entries are still `Pending`.
- Stale answers: `stash_event` returns early unless `entries == Loading::Pending(id)` (`stash.rs:99`). `claims` is at `:135`.
- **Sites to touch for a new job and a new overlay:**
  - Command variant: `update/events/command.rs:162`.
  - Task variant: `update/events.rs:190`.
  - Product variant: `update/events.rs:428`.
  - Answer arm: `update/mod.rs:227`, routing to a `stash_answer`-style fn (`mod.rs:456`).
  - Exhaustive Task list: `mod.rs:521`, in `started` (`:494`).
  - Exhaustive Product list: `mod.rs:872`, in `took_fact` (`:824`).
  - Read-not-write list: `update/place/write.rs:51` (`is_write`).
  - Labels: `event_loop/label.rs:96` and `:148`.
  - Runner: dispatch at `jobs/runner.rs:1161`; job fn at `:1011-1021` (`queue_as(id, Some(Task::..), ..)`, checks `cancel` first).
  - Job body: `jobs/stash.rs:21-30` (`repos.shared()?.handle()` → the read, errors mapped to `String`).
  - Overlay enum: `update/state.rs:654-671`.
  - Precedence chain: `state.rs:614-645`.
  - Frame match: `view/frame.rs:153-155`.
  - Hint context: `view/rows.rs:607-622` (`overlay_context`; returning `None` is allowed, as `CommitEditor` does).
  - Frame tests: `view/frame/tests.rs:293`, `:319`, `:583`.
  - Docs: `MAP.md:126`, `:127`, `:213`, `:891`.

## 2. Key routing
- Entry is `update/mod.rs:961-986`, in this order: credential, name prompt, then search (`/ n N`) only if `search_prompt.is_some() || !modal_owns_keys()` (`:980`), then `keys::key_event`.
- `key_event` (`update/keys.rs:129`) runs in this order:
  1. `drag`, boxes (confirm and preview), finish, resolver, banner, place (`:146-151`).
  2. Context menu (`:162`).
  3. Commit editor (`:167-175`).
  4. `u`/`U`/Ctrl+Z, gated on `!text_field_open()` (`:193`).
  5. Esc stops a running write (`:204`).
  6. Esc dismisses a failure, gated on `!text_field_open()` (`:213`).
  7. `pane_and_overlay_key` (`:243-252`): journal/stash go before `detail_key` while the graph has focus, and after the detail pane and map otherwise.
  8. `graph_key` (`:281`).
- `text_field_open` is `state.rs:775`; `modal_owns_keys` is `:804` (it ORs in `text_field_open`).
- Test `slash_belongs_to_every_modal_that_is_up` is `update/search/tests.rs:171`. It sweeps `every_modal()` (`:52`), which is `text_fields()` (`:59-89+`) plus `boxes()`. Each entry is `(name, state, Option<fn reading the field text>)`; add the picker's state to `text_fields`.

## 3. Hide-under-preview
- Journal: `enter` (`update/journal.rs:297-305`) calls `open_preview(.., RevertTo, true, ..)`; if that returns true it sets `journal.shown = false`, and `selected` is kept.
- Restore on Esc: `close_preview` (`preview.rs:799-811`) and the refused path `refused` (`preview.rs:690-695`) set `journal.shown = true` when the field `PreviewState::return_to_journal` (`preview.rs:212`) is set.
- Stash does **not** hide. `operate_on_selected` passes `false` (`stash.rs:468-477`); the list stays shown and `draw_stash` draws the preview over it (`view/frame/overlays.rs:66`).

## 4. Switch
- `pub fn switch_to(state, target: SwitchTarget, opened_by: &'static str, next_id) -> Vec<Command>` (`update/switch.rs:57`). `switch_into` (`:71`) returns a bool: whether it opened or launched.
- Its cases:
  - Refs not read: `Status::Waiting("still reading the refs — try again in a moment")` (`:80-84`).
  - Detached target, or tracked edits (`previews` at `:113`, `has_edits` at `:120`): `open_preview` (`:88`).
  - Otherwise: `launch` for a BranchCreate, or `launch_unstoppable` with `OnEdits::Refuse`; a refusal is worded by `say_if_refused` (`:98-111`).
  - A `DirtyTree` refusal on the instant launch reopens as a preview (`instant_refused`, `:180`).
- `pub(crate) fn preview_refusal(state: &AppState) -> Option<String>` (`preview.rs:295`) returns:
  - `operate::in_flight` → `InFlight::words()` = `"wait for {running} to finish"` (`operate.rs:175-190`); otherwise
  - `in_progress` → `undo::live_refusal_words` = `"finish or abort the {kind} first"` (`undo.rs:156`).
- `open_preview` (`preview.rs:315`) returns false, with no job, if a preview is already up or on a refusal (writes `Status::Sticky`).
- `operate::operation_in_flight` is `operate.rs:158`.

## 5. `seek_selected`
See C3: `fn seek_selected(next, commands)`, private, tied to `MapState`.

## 6. Drawing
- `draw_stash(session, state, frame, area) -> Option<Rect>` (`view/frame/overlays.rs:46`). It builds `stash_view(state, now)` (`view/stash.rs:22`) into `graft_ui::stash_screen::{StashView (:70), StashScreen{view,theme} (:97), impl Widget (:358)}`.
- The same fn uses `render_overlay_with_detail(..,"Stash")` when the detail pane is shown, then `draw_preview` and `draw_context_menu`.
- Journal equivalent: `overlays.rs:17`.
- Called from `view/frame.rs:155` with the graph `area`.
- Modules: `graft-ui/src/lib.rs:24` and `:30`; `view/mod.rs:25`, `:32`, `:48`.
- Age: `graft_ui::graph_screen::columns::relative_date(then, now)` (`columns.rs:59`, e.g. `2h`); `relative_date_long` (`:72`); private wrappers `age()` in `view/stash.rs:76` (adds ` ago`) and `view/journal.rs:94`.
- Who: `columns::initials(name)` (`columns.rs:18`).

## 7. Tests
- App level: `tests/it/stash_by_key.rs:50-64`, `overlay()`. It calls `App::open(repo)`, `app.pump(state, what, pred)`, then `app.key(&state, KeyCode::Char('S'))`, then pumps until `entries` is not `Idle`/`Pending`.
- Harness `tests/it/app_harness.rs`:
  - `App::open` is `:112`. It runs a real `JobRunner` pool, so jobs are **asynchronous** and `pump` waits on events up to `DEADLINE` (`:232`).
  - `press`/`key` are `:169`/`:177`; `settle_write` is `:246`; `git()` is `:77`.
  - `press_for_commands(state, key) -> (AppState, Vec<Command>)` (`:71`) is pure, for asserting commands.
- Unit level: `update/journal.rs:381` `press` helper.
- `m7_walk`:
  - `noted_keys(id)` (`tests/it/m7_walk.rs:22`) parses the fence in `docs/design-notes/m7-power-tools.md` through `m5_walk::parse_counts`.
  - A walk pushes each key onto `route` and asserts `route.len() <= noted_keys(..)` (`m7_walk.rs:61-79`).
  - Submodules are listed at `:4-9`; `m7_walk/switch.rs` is the closest example.
- Captures: `check_shot("m7/<name>.txt", body)` (`tests/it/captures.rs:171`) → `graft_fixtures::capture`, which asserts on every run and rewrites only under `GRAFT_UPDATE_SHOTS=1`, still failing that run (`graft-fixtures/src/capture.rs:50`).
  - Frame text comes from `frame_at(state, cols, rows, clock)` (`captures.rs:96`); `settled_frame` is `:184`.
  - Example: `m7_walk/switch.rs:171`.

## 8. Hints
- The graph tail list, with S/J/M, is `graft-ui/src/graph_screen/hints/lists.rs:97-118` (`TAIL`).
- `GRAPH_ONLY = ["S","J","M","O"]` is `graft-ui/src/graph_screen/hints.rs:494`.
- The test is `update/tests.rs:5462`. A pair in `keys/bound.rs` `BOUND_ACTIONS` (`:38`; S/J at `:115-116`) must be either Live in `every_hint()` or in `NEVER_HINTED` (`update/tests.rs:5417`).
- So hinting `C` live needs `lists.rs`; the alternative is a `NEVER_HINTED` row with a reason, which is a spec question.
- `C` is claimed today only inside the commit editor (`commit_editor.rs:608`, Ctrl+Shift+C). The keymap gives graph `C` to the picker (`02-keymap.md:204`).

Could not confirm: whether `confirm_preview` touches `return_to_journal` (the grep shows only the two restores).
