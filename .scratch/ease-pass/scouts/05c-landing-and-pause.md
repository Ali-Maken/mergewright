# Scout 05c: clash-stop landing, esc pause, banner ⏏ (at 4462bfe0)

Paths are under `crates/graft/src/update/` unless they say otherwise.

## 1. Where a clash stop lands
- **Graft-begun stop (merge `m … y`, rebase, cherry-pick, revert):** the job's `Conflicted` answer goes to `operate/outcome.rs:419` and then `stopped()` at `:170`. That function sets `banner_focus = stop_takes_focus(..)` (`:211`), then **`land_in_resolver`** (`:224`) and `edit_stop_lands_on_head` (`:225`).
- `land_in_resolver` is at `banner/stop.rs:67-92`. It opens `open_resolver(first, OpenedFrom::Banner)`, or does nothing when there are no paths, a resolver is already open, or a box is open.
- **Already built: every Graft-begun clash stop opens the resolver as `Banner`** (m6-conflicts/18). It is not kind-specific, so all four kinds take it. I did not confirm with a fixture that a revert goes through `outcome.rs:419`.
- **Later stops land through the same function:** `banner/resume/answers.rs:74` (a stop after `c`), and `banner/pressed.rs:152` (after `git --continue` in the shell).
- **Stash put-back clash:** this is a different site. `clash.rs:252` calls `resolver/open.rs:52 land_on_clash`, which opens with `OpenedFrom::Pane` (`:70`) on the changes pane.
- **Relaunch:** `relaunch.rs:49 land_on_live_operation` sets the banner focus (`:85-88`). It then either goes to `relaunch_lands_on_head` (an edit stop, `:105`) or to `land_on_the_work` (`:107`, `:191`), which puts the changes pane on the first Unmerged row. It **never opens the resolver**.

## 2. Opening the resolver
- `resolver/open.rs:18 open_resolver(state, commands, path, from, next_id)` refuses in these cases:
  - a resolver is already open: `:25`
  - a box is open: `:88`
  - `operating`: `:95`
- Otherwise `start` (`:75`) runs `ResolverState::opening` (`resolver.rs:252`) and pushes `Command::OpenConflict`. It also calls `files::reread_set` (`:41`).
- **The file read is async.** `answer.rs:167` takes `Product::ConflictOpened` to `resolver::conflict_opened`. The cursor is set there: `answers.rs:90-124` uses `next_open_in_file` (`picks.rs:273`) for a fresh read and sets `open.cursor_block` (`:124`). The set-wide `next_open` is at `picks.rs:280` (used by `walk.rs:124`).
- `OpenedFrom` (`resolver.rs:126`) has two variants, `Banner` and `Pane`.
  - Set to `Banner` at: `banner/stop.rs:90`, `banner/focus_keys.rs:135`, `resolver/unpick.rs:221`.
  - Set to `Pane` at: `detail/keys.rs:199`, `resolver/open.rs:70`.
  - Read at: `resolver/leave.rs:54`, `resolver/files.rs:187`, `resolved.rs:93`, `unpick.rs:312`, `restart.rs:178`. The last four copy it forward when the resolver reopens; I did not read each one.

## 3. `esc` from the resolver
- `leave.rs:41 escape` refuses while takes are still queued. Otherwise it calls `close` (`:50`): the resolver is dropped; if the resolver was opened as `Banner`, `banner_focus = banner_view().is_some()` (`:54-58`). A sticky status goes back to Ready.
- With `Pane`, the pane focus stays where it was.

## 4. Banner hint context
- The context is picked in exactly one place: `view/rows.rs:578 banner_context` returns `Banner {skippable, conflicted, merge}`. Its callers are `rows.rs:545` and `view/frame/bars.rs:244`.
- **No app code builds `BannerPaused`.**
- Definition: `graft-ui/.../hints/context.rs:127`. Words: `lists.rs:329-341` is `banner_list(false,true,merge)` with ⏎ renamed to `back to the clashes`.
- `update/tests/dead_keys.rs:278-283` still puts `BannerPaused` in the `None` arm.

## 5. Banner `⏎`
- It is in `banner/focus_keys.rs:126-140`, not `pressed.rs`: `first_conflicted` → `open_resolver(.., Banner)`, else `nothing_conflicted`.
- `pressed.rs:71` is inside `continue_pressed` (`:17`), the `c` path.

## 6. `u` mid-operation
- Words: `undo.rs:262` `"finish or abort the {kind} first"`, built by `live_refusal` (`:252`). The `banner/focus_keys.rs` `undo_keys` route also leads there. I did not trace the graph-side caller line.
- §2.6a `u` paragraph (`docs/design/02-keymap.md:409-418`): "**`u` and `U` are refused while an operation is live**, focused or not: `finish or abort the <kind> first · ⇥ banner` … On the focused banner the reason stands alone … **One exception**: in a merge begun in the shell, `u`/`U` undo or redo a step Graft journaled inside that same merge …"

## 7. `edit` stop
- `stop_takes_focus` (`banner/stop.rs:148`) is **false** at an edit stop. **The banner is not focused**, and `edit_stop_lands_on_head` gives the graph the keyboard on HEAD (`:310`). Relaunch does the same (`relaunch.rs:85-105`).

## 8. Tests that pin the banner-first landing
- `tests/it/conflict_interim.rs:153 stopped_on_the_conflict` = `landed_in_the_resolver` (`:164`) + `Esc`, then asserts banner_focus (`:156-157`). So the "⏎ resolve" assertions after it are **post-esc** bars:
  - `:910` (relaunch)
  - `:1076`, `:1132` (`relaunch_lands_on_the_conflict` `:1063`, `q_at_a_stop…` `:1111`)
  - `:517`
  - `:1813` / `capture_banner_bar_with_q` `:1820`
- `conflict_interim/landing_shots.rs`: `capture_a_merge_that_stops` `:50`, `esc_from_the_landing_shows_the_banner_row_on_a_real_merge` `:92`, `capture_after_esc_on_the_banner` `:121`, `capture_a_launch_into_a_shell_merge` `:143`, `capture_after_continue_on_a_rebase` `:182`.
- `m8_walk/teaches.rs:191 resolve_is_on_the_stop_banner_bar` goes through `stopped_on_the_conflict`, so it is a post-esc bar.
- Others found by grep, not read: `m5_walk/conflicts.rs`, `m5_walk/regressions.rs`.
- `view/tour.rs:107-117` already says `y` "lands in the resolver". No tour line says a stop lands on the banner.

## 9. graft-ops
- `Opened` (`graft-ops/src/conflict.rs:40`): `{file: Option<ConflictFile>, how: UnmergedHow, binary: bool}`.
- `UnmergedHow` (`graft-git/src/status.rs:182`): BothModified, BothAdded, BothDeleted, AddedByUs, AddedByThem, DeletedByUs, DeletedByThem.
- **`FileState {Open{open}, Solved, Done}` and `file_state` already exist** at `graft-ops/src/conflict/state.rs:11,30`. They are tested in `state/tests.rs` and used at `apply/resume/shell.rs:145`.
- The app does not call `file_state`. It has its own `RowState` (`resolver/files.rs:23`: Open/AllTaken/Done/WholeFile/Binary/Unread).
- Not found: an app-held per-file `Opened`.

## 10. Line counts / ceilings (`tests/it/over_400.txt`)
| File | Lines | Ceiling |
|---|---|---|
| outcome.rs | 430 | 430 (`:180`) |
| clash.rs | 552 | 552 |
| view/rows.rs | 897 | 897 |
| conflict_interim.rs | 2006 | 2006 |
| ui context.rs | 489 | 489 |
| resolver.rs | 400 | none |
| ui lists.rs | 400 | none |
| stop.rs | 386 | none |
| focus_keys.rs | 381 | none |
| answers.rs (resume) | 321 | none |
| dead_keys.rs | 324 | none |
| undo.rs | 281 | none |
| pressed.rs | 264 | none |
| relaunch.rs | 227 | none |
| landing_shots.rs | 244 | none |
| teaches.rs | 210 | none |
| open.rs | 148 | none |
| tour.rs | 130 | none |
| leave.rs | 64 | none |
| state.rs (ops) | 42 | none |

## Contradictions with 05b Decided / 05c
1. **Q17 a is already in place for Graft-begun stops:** `outcome.rs:224`, `resume/answers.rs:74`, `pressed.rs:152`. The landing site the spec names is wrong: `relaunch.rs:49` is the relaunch path, which by Decided stays on the banner.
2. Spec (05c `:75`): an edit stop lands "on the graph with the banner focused". Code: banner **not** focused (`stop.rs:148`, `outcome.rs:211`).
3. Spec: banner ⏎ is `banner/pressed.rs:71`. Code: it is at `focus_keys.rs:126`.
4. 05c `:71` says `file_state` is not built. It is at `graft-ops/src/conflict/state.rs:30`.
5. The 03b deviation says the `⏎ resolve` captures are first landings. In code most are **post-esc** banner bars (`conflict_interim.rs:155`), so `BannerPaused` would change them. Only the relaunch ones (`:910`, `:1076`, `:1132`) are true first landings.
6. The `OpenedFrom::Banner` value is carried forward at `files.rs:187`, `resolved.rs:93`, `unpick.rs:312` and `restart.rs:178`. Could not confirm whether any of these drive behaviour beyond `leave.rs:54`.
