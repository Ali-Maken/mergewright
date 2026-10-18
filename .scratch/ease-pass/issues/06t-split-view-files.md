# 06t — split the four `view/` files 06 grew past 400 (review ease-pass-06 M6, view half)

Status: done 2026-10-18. Was: ready (planner spec-only pass 2026-10-18, second: fitted to scout `.scratch/ease-pass/scouts/06t-split-view-files.md` at 2352d87f; its 5 contradictions ruled in Decided). Was: draft.
Blocked by: none open — 06h merged (2352d87f). 06i dropped: its Scope (06i:47) holds `update::palette` and graft-ui `palette`, no `view/` file.
**Runs alone on these four files.** 07, 07b, 09 and 13 name `view::entries`, `view::rows` and the `view::strips` readers, so they wait for this ticket. 11b holds `view/resolver*` only (11b:22), which is disjoint. Runs beside 06s (disjoint: `update/`).
Build with: Sonnet (builder-sonnet). Two files go by test moves alone, two by plain moves into child modules, and the one test-path line is ruled below. No structural choice is left (scout's call).
Needs: none (moves only).
Review: `docs/reviews/ease-pass-06.md` M6. User, 2026-10-18 (through the conductor): code modular so future features are easy to add — split, don't raise the ratchet.

## Read first
- `docs/reviews/ease-pass-06.md` M6.
- PROMPT.md §6.2. `ARCHITECTURE.md` "Keeping crates deep".
- `~/.claude/plugins/cache/claude-plugins-official/mattpocock-skills/1.2.3/skills/engineering/codebase-design/SKILL.md`.
- `crates/mergewright/MAP.md`: `view`, `view::rows`, `view::radar`, `view::entries`, `view::detail`.
- The scout's file (line map at 2352d87f).

## Decided here (the scout's contradictions)
- **C1 — `tests/it/stop_ladder_readers.rs:57` follows the move.**
  - That entry is a registry of which file reads `stop_wordings`. When `stopped_line` (:515) moves with its family, the one path string changes to the new file. The number of entries and every assertion stay the same.
  - This is the same allowance as `use` lines and path-keyed allow-lists: the reader is the same fn and only its address changed.
  - NOTES shows the one-line diff. Keeping `stopped_line` behind in `rows.rs` just to spare the string is **rejected**: it would leave one hint fn outside its family.
- **`view/rows.rs` (843 non-test):**
  - The hint-context group (`box_context` :491 … `keep_for` :830, ~340 lines; includes `stopped_line` :515 and the `PlainRow` pick in `hint_context` :765) → `rows/hint_context.rs`.
  - Header and filters (`filters` :142 … `rows_for_frame` :203) plus location (:226–270) → `rows/header.rs`.
  - The window cut (`reveal_index` :405 … `compaction` :447) and footer lines (:351–404) → by job (`rows/window.rs`, `rows/footer.rs`). The builder may merge these two if one would be under ~40 lines.
  - `graph_view`, `opening_view` (with the `PlainRow` pick :308) and `walk_view` stay.
  - Child modules reach the parent's private fns through `super::`, and `rows/context.rs`'s imports back from `rows` keep working.
- **`view/entries.rs` (428, no tests):** the cells/badges part (`row_entry` :291–371, `stopped_here` :372, `badge_view` :383–428) → `entries/cells.rs`, so 07's rewrite lands in one file and `entries.rs` doesn't regrow. The uncommitted row stays. 06r's node helper is already `view/node.rs`; nothing to do there.
- **C2 — `view/detail.rs`:** there is no radar-lines section. Non-test code is 325 lines, so moving `mod tests` (:326–407) to `detail/tests.rs` is the whole split.
- **C3 — `view/radar.rs`:** non-test code is 250 lines, so moving `mod tests` (:251–732) to `radar/tests.rs` is the whole split. The three-jobs seam is dropped (not needed at 250 lines).
- **C5 — the review's cite `entries.rs:300-308,341-346`** falls inside `row_entry`. NOTES maps it to `entries/cells.rs`; no meaning is inferred.
- **Visibility:** a moved fn the old file still calls may become `pub(super)`, which reaches only the scope that saw it before. Nothing becomes `pub(crate)` or `pub` that was narrower. CONTEXT.md names.
- Moving inline tests to a sibling `tests.rs` is allowed **in addition**, and as the whole split only where non-test code is under 400 (radar, detail).

## Scope
In:
- the four files and new modules under `crates/mergewright/src/view/{rows,entries,radar,detail}/`;
- `crates/mergewright/tests/it/over_400.txt` (four rows removed, none added);
- `crates/mergewright/tests/it/stop_ladder_readers.rs:57` (the one path string, C1);
- `crates/mergewright/MAP.md`.
Out: any behaviour, name or signature change; `update/` (06s); `view::strips` (13); `rows/context.rs`, `rows/fold.rs`, `rows/clash_bars.rs` (not over 400; 07b edits `context.rs`); `view/node.rs`.

## Touches (check each, report how)
- Every `use` of a moved item; the `super::entries::{Thread, thread, uncommitted_shown, visible_rows}` import (`rows.rs:22`).
- MAP coverage; `no_literal_keys`/sanitisation allow-lists keyed by path; `stop_ladder_readers` (C1).
- Captures: byte-equal, no update flag. Clippy size lints.
- Comments that cite these files: `view/tests.rs:4942`, `update/context_menu.rs:48` (entries.rs), `ghostty_shot.rs:179` (rows.rs). Update each to the new home.
- In-flight specs: 07 (`view::entries` cells and badges → `entries/cells.rs`), 07b (`view/rows.rs:308` stays, `:765` → `rows/hint_context.rs`), 09, 13, 14. NOTES gives each old line's new home.

## Acceptance
Each tick names its test inline.
- [x] Each of the four files under 400 lines; rows gone from `over_400.txt`; none added or raised. — it `file_size` ratchet test; `wc -l` and `git diff` in NOTES  (ticked: `file_size::every_file_over_400_lines_is_listed_with_its_count` + `the_size_list_has_no_stale_rows` green; sizes in NOTES)
- [x] Nextest count equal; in tests, only `use` lines and the one `stop_ladder_readers.rs:57` path changed; every capture test passes without `MWR_UPDATE_SHOTS`. — counts and `git diff --stat` in NOTES; `stop_ladder_readers` green  (ticked: 6947 run / 6947 passed, same 6947 as the run before the radar test split; `stop_ladder_readers` green; no MWR_UPDATE_SHOTS; 19 #[test] removed = 19 moved)
- [x] No visibility widened beyond `pub(super)` on moved items. — `git diff | grep '^+.*pub'` reviewed in NOTES  (ticked: diff `^+.*pub` shows only re-exports at old visibility; children use pub(super) / pub(in crate::view); clippy green)
- [x] MAP.md has each new module. — it `map_coverage`  (ticked: `map_coverage` green)
- [x] NOTES: the seam per file and why; the old-line → new-home table for 07, 07b, 09, 13, 14 and the review's cite.  (ticked: NOTES below)

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~50k (two files are test moves). Split point: `rows.rs` + `entries.rs`, then the two test moves.

## NOTES (builder, 2026-10-18)
Scout used: `.scratch/ease-pass/scouts/06t-split-view-files.md` (line map matched main tip 4fc96acf).

Sizes after (wc -l): rows.rs 223, entries.rs 285, radar.rs 252, detail.rs 327; new rows/{header 149, footer 58, window 98, hint_context 383}, entries/cells.rs 157, radar/tests.rs 260 + radar/tests/evil.rs 225, detail/tests.rs ~80. `over_400.txt`: four rows removed, none added.

Seams:
- rows.rs → `header` (filters…home_relative), `footer` (search/compare/order lines), `window` (reveal_index…compaction; kept apart from footer, both > 40 lines), `hint_context` (box_context…keep_for). Old paths kept by re-exports in rows.rs at their old visibility; former `pub(super)` items are `pub(in crate::view)` in the child (same reach). Private fns rows.rs calls became `pub(super)` (reach: rows only).
- entries.rs → `entries/cells.rs` (row_entry, stopped_here, badge_view); `entries::badge_view` re-exported.
- radar, detail: `mod tests` moved to `tests.rs`. radar's tests were 479 lines, which the ratchet counts, so the evil-probe tests went to `radar/tests/evil.rs` (moves only, `use super::*`).
- Test edits: `rows/tests.rs` use lines only (`hint_context`, `Head`, `RepositoryFacts`, `Path(Buf)` the parent no longer imports); C1 path `stop_ladder_readers.rs:57`: `"mergewright/src/view/rows.rs"` → `"mergewright/src/view/rows/hint_context.rs"`.
- Comments updated: `ghostty_shot.rs:179` → `view/rows/window.rs`; `update/context_menu.rs:48` → `view/entries/cells.rs`. `view/tests.rs:4942` cites drop-target paint (`drag_paint`), which stays in entries.rs: unchanged. Not updated (out of scope, comment-only): `update/geometry.rs:485` says `view::entries::row_entry` (now `entries::cells::row_entry`).

Old line → new home (at 2352d87f):
| old | new |
|---|---|
| rows.rs:142–270 | rows/header.rs |
| rows.rs:308 (PlainRow pick in opening_view) | stays rows.rs (07b) |
| rows.rs:351–404 | rows/footer.rs |
| rows.rs:405–490 | rows/window.rs |
| rows.rs:491–842 incl. :515 stopped_line, :765 PlainRow | rows/hint_context.rs (07b) |
| entries.rs:291–428 (row_entry, stopped_here, badge_view) | entries/cells.rs (07; review cite :300-308,341-346 is inside row_entry) |
| entries.rs:1–290 | stay entries.rs (09, 13, 14 readers of Thread/visible_rows/uncommitted unchanged) |
| radar.rs:251–732, detail.rs:326–407 | radar/tests.rs (+tests/evil.rs), detail/tests.rs |
`view::strips` untouched (13).
