# Scout 13: resolver-file-list (main @ 72911cc)

## Paths after ticket 23's split
- graft-ui: `conflict_resolver.rs` (types, `ResolverScreen`), `conflict_resolver/{draw,layout,names,tests}.rs`. No `files.rs` yet.
- graft app: `update/resolver.rs` + `update/resolver/{answers,base,geometry,names,restart,scroll,take,walk,whole_file,tests}.rs`; `view/resolver.rs`; `view/frame/conflicts.rs`.
- graft-ops: `conflict/set.rs` (`ConflictSet`, `operation_conflicts`), re-exported at `conflict.rs:32`.

## State
- `ResolverState` `update/resolver.rs:85-144`: path, opened_from, file, whole_file, how, cursor_block, top, base_shown, job, reading, base_job, staging, restarting, notice. No `files`/`file_cursor`.
- `ConflictSet` `graft-ops/src/conflict/set.rs:27-36`: only `paths: Vec<PathBuf>` + `recorded: bool`. No per-file state. So Done/Unread/AllTaken/WholeFile must be derived app-side (working_tree + open doc).
- No `Command::ReadConflictSet` / `Product::ConflictSet` exist (grep: none). `Command::OpenConflict(JobId, PathBuf)` at `update/events.rs:597`.

## Draw
- `DocView` `conflict_resolver.rs:87-105`: `title`, `names` (ticket 10 landed), regions, cursor_block, base_shown, notice. `HEADER_ROWS = 2` (`:58`: title + notice row). Title drawn `:265-292`; body starts at `area.y + HEADER_ROWS` `:246-250`.
- Title built by `resolver_title` `view/resolver.rs:170-189` (`path · file i of n · k of n conflicts open`); file index from `banner::unmerged_paths` `view/resolver.rs:31-35,64`.
- Rect split (Q3): `view/frame/conflicts.rs:28-56` — `draw_resolver_banner` takes `banner_rows` off the top, then `document = body.height - 1` (footer). The widget itself splits title/notice (`HEADER_ROWS`). `layout(&doc, body.width)` `:37` — no arrangement arg.
- `MIN_COLUMNS 60`/`MIN_ROWS 10` `conflict_resolver.rs:79,83`; `SIDE_BY_SIDE_MIN 100` `:54`.

## Answers to the four questions
1. `resolved_advance` `update/resolver/answers.rs:216-252`: gated on `open.staging == id`; `remaining_conflicted_file` `walk.rs:108-114` = next unmerged path after `done` in `working_tree` order (`walk.rs:95-100`), else first other unmerged path (wrap). Source is `banner::unmerged_paths` `banner.rs:71-81`. Notice today: `all conflicts resolved · c continue` / `all conflicts resolved` (`answers.rs:245-251`).
2. `resolver_mouse` `scroll.rs:24-39` → `click` `:43-60`: `row.checked_sub(body_y(state))`, `body_y = banner_rows + HEADER_ROWS` (`geometry.rs:40-42`), then `row_at(&layout, top, body_row)`. Row above body → ignored.
3. See Draw: `view/frame/conflicts.rs:34-43`.
4. Yes: the `OpenConflict` job reads status itself, `jobs/conflict.rs:100-113` (`graft_git::status::read`), `not_unmerged` flag `:85,116`.

## Keys
13 binds no key (click only). Keymap §2.6 `docs/design/02-keymap.md:260-273`: `j`/`k` = P7 (ticket 17), `O`/`T` = P6. Existing resolver keys `update/resolver.rs:283-296`; unmatched plain keys fall to `scroll_key`.

## Contradictions / flags
- **`arrangement`/`Arrangement::ThreePane` do not exist in code** (grep empty). Defined in design note `m6-conflicts.md:522-527` and built by ticket 14 (`14-resolver-three-panes.md:23`), which is *blocked by* 13. 13 §3 uses `arrangement(frame_columns)`.
- `fits` `geometry.rs:60-67` counts banner + footer only, not `HEADER_ROWS`; 13 says `fits` counts strip + rule rows — changes the floor semantics.
- Keymap `02-keymap.md:269`: "resolver stays open after the last file is done (P8)" vs 13 §6 "today's close to the banner". P8 is ticket 19 Part B (`19-*.md:5`), so not a code contradiction, but doc and 13 disagree until 19.
- Stash-pop "pane gets `every file done ✓`": today status line, not pane (`answers.rs:251`); could not confirm what "pane" means.

## Overlap with 24
24 moves `banner.rs:574-793` (stop/landing) — not `unmerged_paths` (`:71`), which 13 relies on. Both crates graft + graft-ui; 24 edits `preview.rs:459-468` (13 doesn't). Low textual overlap, but same crate set → not concurrent per ADR-0036.
